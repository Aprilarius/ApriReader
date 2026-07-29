#[cfg(any(feature = "steam-build", test))]
use crate::statistics::ACHIEVEMENTS;
#[cfg(any(feature = "steam-build", test))]
use rusqlite::params;
use rusqlite::Connection;
use serde::Serialize;
#[cfg(any(feature = "steam-build", test))]
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[cfg(any(feature = "steam-build", test))]
const MAX_SYNC_ERROR_LENGTH: usize = 500;

#[derive(Debug, Error)]
pub enum SteamError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Steam synchronization is unavailable: {0}")]
    Unavailable(String),
    #[cfg(any(feature = "steam-build", test))]
    #[error("Steam synchronization failed: {0}")]
    Provider(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamIntegrationStatus {
    pub build_profile: &'static str,
    pub bridge_installed: bool,
    pub provider_available: bool,
    pub overlay_enabled: Option<bool>,
    pub pending_unlocks: i64,
    pub synced_unlocks: i64,
    pub last_attempt_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamSyncResult {
    pub attempted: usize,
    pub synchronized: usize,
    pub pending: i64,
    pub overlay_enabled: Option<bool>,
}

#[cfg(any(feature = "steam-build", test))]
#[derive(Debug, Clone, Copy)]
struct ProviderReceipt {
    overlay_enabled: Option<bool>,
}

#[cfg(any(feature = "steam-build", test))]
trait SteamAchievementProvider {
    fn synchronize(&mut self, achievement_ids: &[String]) -> Result<ProviderReceipt, String>;
}

pub fn status(connection: &Connection) -> Result<SteamIntegrationStatus, SteamError> {
    let (pending, synced, last_attempt_at, last_error) = queue_status(connection)?;
    let bridge_installed = bridge_path().is_some_and(|path| path.is_file());
    #[cfg(all(feature = "steam-build", target_os = "windows"))]
    let probe = SteamBridgeProvider::load().map(|provider| provider.overlay_enabled());
    #[cfg(not(all(feature = "steam-build", target_os = "windows")))]
    let probe: Result<Option<bool>, String> =
        Err("this public build does not include the Steam profile".to_owned());
    let (provider_available, overlay_enabled) = match probe {
        Ok(overlay) => (true, overlay),
        Err(_) => (false, None),
    };
    Ok(SteamIntegrationStatus {
        build_profile: if cfg!(feature = "steam-build") {
            "steam"
        } else {
            "github"
        },
        bridge_installed,
        provider_available,
        overlay_enabled,
        pending_unlocks: pending,
        synced_unlocks: synced,
        last_attempt_at,
        last_error,
    })
}

pub fn synchronize(connection: &Connection) -> Result<SteamSyncResult, SteamError> {
    #[cfg(all(feature = "steam-build", target_os = "windows"))]
    {
        let mut provider = SteamBridgeProvider::load().map_err(SteamError::Unavailable)?;
        synchronize_with_provider(connection, &mut provider)
    }
    #[cfg(not(all(feature = "steam-build", target_os = "windows")))]
    {
        let _ = connection;
        Err(SteamError::Unavailable(
            "use the protected Steam build with aprireader_steam_bridge.dll".to_owned(),
        ))
    }
}

#[cfg(any(feature = "steam-build", test))]
fn synchronize_with_provider(
    connection: &Connection,
    provider: &mut impl SteamAchievementProvider,
) -> Result<SteamSyncResult, SteamError> {
    let mut statement = connection.prepare(
        "SELECT achievement_id FROM achievement_sync_queue
         WHERE synced_at IS NULL
         ORDER BY unlocked_at, achievement_id",
    )?;
    let pending = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    let known_ids = ACHIEVEMENTS
        .iter()
        .map(|definition| definition.id)
        .collect::<std::collections::HashSet<_>>();
    let pending = pending
        .into_iter()
        .filter(|id| known_ids.contains(id.as_str()))
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return Ok(SteamSyncResult {
            attempted: 0,
            synchronized: 0,
            pending: 0,
            overlay_enabled: None,
        });
    }
    let now = unix_seconds();
    match provider.synchronize(&pending) {
        Ok(receipt) => {
            let transaction = connection.unchecked_transaction()?;
            for id in &pending {
                transaction.execute(
                    "UPDATE achievement_sync_queue
                     SET attempts = attempts + 1, last_attempt_at = ?1,
                         last_error = NULL, synced_at = ?1
                     WHERE achievement_id = ?2 AND synced_at IS NULL",
                    params![now, id],
                )?;
            }
            transaction.commit()?;
            Ok(SteamSyncResult {
                attempted: pending.len(),
                synchronized: pending.len(),
                pending: pending_count(connection)?,
                overlay_enabled: receipt.overlay_enabled,
            })
        }
        Err(error) => {
            let bounded = error
                .chars()
                .take(MAX_SYNC_ERROR_LENGTH)
                .collect::<String>();
            let transaction = connection.unchecked_transaction()?;
            for id in &pending {
                transaction.execute(
                    "UPDATE achievement_sync_queue
                     SET attempts = attempts + 1, last_attempt_at = ?1, last_error = ?2
                     WHERE achievement_id = ?3 AND synced_at IS NULL",
                    params![now, bounded, id],
                )?;
            }
            transaction.commit()?;
            Err(SteamError::Provider(bounded))
        }
    }
}

fn queue_status(
    connection: &Connection,
) -> Result<(i64, i64, Option<i64>, Option<String>), rusqlite::Error> {
    connection.query_row(
        "SELECT
            SUM(CASE WHEN synced_at IS NULL THEN 1 ELSE 0 END),
            SUM(CASE WHEN synced_at IS NOT NULL THEN 1 ELSE 0 END),
            MAX(last_attempt_at),
            (SELECT last_error FROM achievement_sync_queue
             WHERE last_error IS NOT NULL ORDER BY last_attempt_at DESC LIMIT 1)
         FROM achievement_sync_queue",
        [],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get(2)?,
                row.get(3)?,
            ))
        },
    )
}

#[cfg(any(feature = "steam-build", test))]
fn pending_count(connection: &Connection) -> Result<i64, rusqlite::Error> {
    connection.query_row(
        "SELECT COUNT(*) FROM achievement_sync_queue WHERE synced_at IS NULL",
        [],
        |row| row.get(0),
    )
}

fn bridge_path() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|directory| directory.join("aprireader_steam_bridge.dll"))
}

#[cfg(any(feature = "steam-build", test))]
fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[cfg(all(feature = "steam-build", target_os = "windows"))]
struct SteamBridgeProvider {
    module: *mut std::ffi::c_void,
    unlock: unsafe extern "C" fn(*const std::ffi::c_char) -> i32,
    store: unsafe extern "C" fn() -> i32,
    overlay: unsafe extern "C" fn() -> i32,
    shutdown: unsafe extern "C" fn(),
}

#[cfg(all(feature = "steam-build", target_os = "windows"))]
impl SteamBridgeProvider {
    fn load() -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        let path = bridge_path().ok_or_else(|| "cannot locate the executable".to_owned())?;
        if !path.is_file() {
            return Err("aprireader_steam_bridge.dll is not installed".to_owned());
        }
        let wide = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let module = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
        };
        if module.is_null() {
            return Err("the protected Steam bridge could not be loaded".to_owned());
        }
        let init = unsafe {
            load_symbol::<unsafe extern "C" fn() -> i32>(module, b"aprireader_steam_init\0")
        };
        let unlock = unsafe {
            load_symbol::<unsafe extern "C" fn(*const std::ffi::c_char) -> i32>(
                module,
                b"aprireader_steam_unlock\0",
            )
        };
        let store = unsafe {
            load_symbol::<unsafe extern "C" fn() -> i32>(module, b"aprireader_steam_store\0")
        };
        let overlay = unsafe {
            load_symbol::<unsafe extern "C" fn() -> i32>(
                module,
                b"aprireader_steam_overlay_enabled\0",
            )
        };
        let shutdown = unsafe {
            load_symbol::<unsafe extern "C" fn()>(module, b"aprireader_steam_shutdown\0")
        };
        let (init, unlock, store, overlay, shutdown) =
            match (init, unlock, store, overlay, shutdown) {
                (Some(init), Some(unlock), Some(store), Some(overlay), Some(shutdown)) => {
                    (init, unlock, store, overlay, shutdown)
                }
                _ => {
                    unsafe { FreeLibrary(module) };
                    return Err("the protected Steam bridge has an incompatible ABI".to_owned());
                }
            };
        if unsafe { init() } != 1 {
            unsafe { FreeLibrary(module) };
            return Err("Steam is not initialized or the user stats are unavailable".to_owned());
        }
        Ok(Self {
            module,
            unlock,
            store,
            overlay,
            shutdown,
        })
    }

    fn overlay_enabled(&self) -> Option<bool> {
        Some(unsafe { (self.overlay)() } == 1)
    }
}

#[cfg(all(feature = "steam-build", target_os = "windows"))]
impl SteamAchievementProvider for SteamBridgeProvider {
    fn synchronize(&mut self, achievement_ids: &[String]) -> Result<ProviderReceipt, String> {
        for id in achievement_ids {
            let id = std::ffi::CString::new(id.as_str())
                .map_err(|_| "an achievement ID is invalid".to_owned())?;
            if unsafe { (self.unlock)(id.as_ptr()) } != 1 {
                return Err("Steam rejected an achievement identifier".to_owned());
            }
        }
        if unsafe { (self.store)() } != 1 {
            return Err("Steam did not confirm StoreStats".to_owned());
        }
        Ok(ProviderReceipt {
            overlay_enabled: self.overlay_enabled(),
        })
    }
}

#[cfg(all(feature = "steam-build", target_os = "windows"))]
impl Drop for SteamBridgeProvider {
    fn drop(&mut self) {
        unsafe {
            (self.shutdown)();
            FreeLibrary(self.module);
        }
    }
}

#[cfg(all(feature = "steam-build", target_os = "windows"))]
unsafe fn load_symbol<T: Copy>(module: *mut std::ffi::c_void, name: &'static [u8]) -> Option<T> {
    let pointer = unsafe { GetProcAddress(module, name.as_ptr().cast()) };
    if pointer.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute_copy(&pointer) })
    }
}

#[cfg(all(feature = "steam-build", target_os = "windows"))]
const LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR: u32 = 0x0000_0100;
#[cfg(all(feature = "steam-build", target_os = "windows"))]
const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 0x0000_1000;

#[cfg(all(feature = "steam-build", target_os = "windows"))]
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryExW(
        file_name: *const u16,
        file: *mut std::ffi::c_void,
        flags: u32,
    ) -> *mut std::ffi::c_void;
    fn GetProcAddress(
        module: *mut std::ffi::c_void,
        name: *const std::ffi::c_char,
    ) -> *mut std::ffi::c_void;
    fn FreeLibrary(module: *mut std::ffi::c_void) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeProvider {
        fail: bool,
        received: Vec<String>,
    }

    impl SteamAchievementProvider for FakeProvider {
        fn synchronize(&mut self, achievement_ids: &[String]) -> Result<ProviderReceipt, String> {
            self.received.extend_from_slice(achievement_ids);
            if self.fail {
                Err("offline".to_owned())
            } else {
                Ok(ProviderReceipt {
                    overlay_enabled: Some(true),
                })
            }
        }
    }

    fn database() -> Connection {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                "CREATE TABLE achievement_sync_queue(
                    achievement_id TEXT PRIMARY KEY,
                    unlocked_at INTEGER NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    last_attempt_at INTEGER,
                    last_error TEXT,
                    synced_at INTEGER
                 ) STRICT;
                 INSERT INTO achievement_sync_queue(achievement_id, unlocked_at)
                 VALUES('first_book_opened', 1), ('first_book_finished', 2);",
            )
            .expect("schema");
        connection
    }

    #[test]
    fn successful_store_marks_each_unlock_once() {
        let connection = database();
        let mut provider = FakeProvider {
            fail: false,
            received: Vec::new(),
        };
        let first = synchronize_with_provider(&connection, &mut provider).expect("sync");
        let second = synchronize_with_provider(&connection, &mut provider).expect("repeat");
        assert_eq!(first.synchronized, 2);
        assert_eq!(second.attempted, 0);
        assert_eq!(provider.received.len(), 2);
        assert_eq!(pending_count(&connection).expect("pending"), 0);
    }

    #[test]
    fn failed_store_keeps_unlocks_pending() {
        let connection = database();
        let mut provider = FakeProvider {
            fail: true,
            received: Vec::new(),
        };
        let error =
            synchronize_with_provider(&connection, &mut provider).expect_err("offline failure");
        assert!(matches!(error, SteamError::Provider(_)));
        assert_eq!(pending_count(&connection).expect("pending"), 2);
        let attempts = connection
            .query_row(
                "SELECT SUM(attempts) FROM achievement_sync_queue",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("attempts");
        assert_eq!(attempts, 2);
    }

    #[test]
    fn unknown_identifiers_never_reach_provider() {
        let connection = database();
        connection
            .execute(
                "INSERT INTO achievement_sync_queue(achievement_id, unlocked_at)
                 VALUES('not_canonical', 3)",
                [],
            )
            .expect("unknown id");
        let mut provider = FakeProvider {
            fail: false,
            received: Vec::new(),
        };
        synchronize_with_provider(&connection, &mut provider).expect("sync");
        assert!(!provider.received.iter().any(|id| id == "not_canonical"));
    }

    #[test]
    fn steam_manifest_uses_exactly_the_canonical_identifiers() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../docs/steam/ACHIEVEMENTS.json"))
                .expect("Steam achievement manifest");
        let manifest_ids = manifest["achievements"]
            .as_array()
            .expect("achievement array")
            .iter()
            .map(|item| item["id"].as_str().expect("achievement id"))
            .collect::<std::collections::HashSet<_>>();
        let canonical_ids = ACHIEVEMENTS
            .iter()
            .map(|definition| definition.id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(manifest_ids, canonical_ids);
    }
}
