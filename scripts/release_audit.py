"""Fail closed on ApriReader's repository-level release security gates."""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN_FILES = {
    "aprireader_steam_bridge.dll",
    "steam_api64.dll",
    "steam_appid.txt",
}
IGNORED_DIRS = {".git", "node_modules", "target", "dist"}
APPROVED_NPM = {
    "@tauri-apps/api": "MIT OR Apache-2.0",
    "@tauri-apps/plugin-dialog": "MIT OR Apache-2.0",
    "@tauri-apps/plugin-opener": "MIT OR Apache-2.0",
    "pdfjs-dist": "Apache-2.0",
    "react": "MIT",
    "react-dom": "MIT",
}
APPROVED_CARGO = {
    "base64": "MIT OR Apache-2.0",
    "quick-xml": "MIT",
    "rars": "MIT OR Apache-2.0",
    "rusqlite": "MIT",
    "serde": "MIT OR Apache-2.0",
    "serde_json": "MIT OR Apache-2.0",
    "sha2": "MIT OR Apache-2.0",
    "tauri": "MIT OR Apache-2.0",
    "tauri-plugin-dialog": "MIT OR Apache-2.0",
    "tauri-plugin-opener": "MIT OR Apache-2.0",
    "tauri-plugin-single-instance": "MIT OR Apache-2.0",
    "thiserror": "MIT OR Apache-2.0",
    "ureq": "MIT OR Apache-2.0",
    "windows": "MIT OR Apache-2.0",
    "zip": "MIT",
}
BUNDLED_FONT_FILES = {
    "CharisSIL-Bold.ttf",
    "CharisSIL-BoldItalic.ttf",
    "CharisSIL-Italic.ttf",
    "CharisSIL-Regular.ttf",
    "IBMPlexSerif-Bold.ttf",
    "IBMPlexSerif-BoldItalic.ttf",
    "IBMPlexSerif-ExtraLight.ttf",
    "IBMPlexSerif-ExtraLightItalic.ttf",
    "IBMPlexSerif-Italic.ttf",
    "IBMPlexSerif-Light.ttf",
    "IBMPlexSerif-LightItalic.ttf",
    "IBMPlexSerif-Medium.ttf",
    "IBMPlexSerif-MediumItalic.ttf",
    "IBMPlexSerif-Regular.ttf",
    "IBMPlexSerif-SemiBold.ttf",
    "IBMPlexSerif-SemiBoldItalic.ttf",
    "IBMPlexSerif-Thin.ttf",
    "IBMPlexSerif-ThinItalic.ttf",
    "Literata-Italic-Variable.ttf",
    "Literata-Variable.ttf",
    "Lora-Italic-Variable.ttf",
    "Lora-Variable.ttf",
    "Merriweather-Italic-Variable.ttf",
    "Merriweather-Variable.ttf",
    "SourceSerif4-Italic-Variable.ttf",
    "SourceSerif4-Variable.ttf",
}
BUNDLED_FONT_LICENSES = {
    "Charis_SIL-OFL.txt",
    "IBM_Plex_Serif-OFL.txt",
    "Literata-OFL.txt",
    "Lora-OFL.txt",
    "Merriweather-OFL.txt",
    "Source_Serif_4-OFL.txt",
}


def fail(message: str) -> None:
    print(f"release audit failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def check_forbidden_files() -> None:
    for path in ROOT.rglob("*"):
        if any(part in IGNORED_DIRS for part in path.relative_to(ROOT).parts):
            continue
        if path.is_file() and path.name.casefold() in FORBIDDEN_FILES:
            fail(f"protected artifact present: {path.relative_to(ROOT)}")


def check_direct_dependencies() -> None:
    package = json.loads((ROOT / "package.json").read_text("utf-8"))
    npm = set(package["dependencies"])
    if npm != set(APPROVED_NPM):
        fail(f"npm dependency review is stale: {sorted(npm ^ set(APPROVED_NPM))}")
    pdfjs_version = package["dependencies"].get("pdfjs-dist", "")
    try:
        pdfjs_parts = tuple(int(part) for part in pdfjs_version.split("."))
    except (AttributeError, ValueError):
        fail("pdfjs-dist must use an exact numeric version")
    if len(pdfjs_parts) != 3 or pdfjs_parts < (6, 2, 108):
        fail("pdfjs-dist is below the reviewed GHSA-hq66-cqwq-w95j fix")

    metadata_source = (ROOT / "src-tauri" / "src" / "metadata.rs").read_text("utf-8")
    if "api.fantlab.ru" in metadata_source:
        fail("retired FantLab metadata endpoint is still present")
    for required in (
        '"https://inventaire.io/api/search"',
        '"https://inventaire.io"',
        'strip_prefix("/img/entities/")',
    ):
        if required not in metadata_source:
            fail(f"reviewed Inventaire boundary is missing: {required}")
    cargo = tomllib.loads((ROOT / "src-tauri" / "Cargo.toml").read_text("utf-8"))
    rust = set(cargo["dependencies"])
    for target in cargo.get("target", {}).values():
        rust.update(target.get("dependencies", {}))
    if rust != set(APPROVED_CARGO):
        fail(f"Cargo dependency review is stale: {sorted(rust ^ set(APPROVED_CARGO))}")


def check_bundled_fonts() -> None:
    font_dir = ROOT / "src" / "assets" / "fonts"
    actual_fonts = {path.name for path in font_dir.glob("*.ttf")}
    if actual_fonts != BUNDLED_FONT_FILES:
        fail(
            "bundled font review is stale: "
            f"{sorted(actual_fonts ^ BUNDLED_FONT_FILES)}"
        )
    total_bytes = 0
    for name in BUNDLED_FONT_FILES:
        path = font_dir / name
        total_bytes += path.stat().st_size
        if path.read_bytes()[:4] not in {b"\x00\x01\x00\x00", b"true"}:
            fail(f"bundled font has an invalid TrueType signature: {name}")
    if total_bytes > 20 * 1024 * 1024:
        fail("bundled font set exceeds the reviewed 20 MB limit")

    license_dir = ROOT / "public" / "licenses" / "fonts"
    actual_licenses = {path.name for path in license_dir.glob("*-OFL.txt")}
    if actual_licenses != BUNDLED_FONT_LICENSES:
        fail(
            "bundled font license review is stale: "
            f"{sorted(actual_licenses ^ BUNDLED_FONT_LICENSES)}"
        )
    for name in BUNDLED_FONT_LICENSES:
        text = (license_dir / name).read_text("utf-8")
        if "SIL OPEN FONT LICENSE Version 1.1" not in text:
            fail(f"bundled font license is not the reviewed OFL 1.1 text: {name}")


def check_tauri_boundary() -> None:
    config = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text("utf-8"))
    security = config["app"]["security"]
    csp = security["csp"]
    if "'unsafe-eval'" in csp or "script-src *" in csp:
        fail("CSP permits dynamic or unrestricted script execution")
    if security["capabilities"] != ["default"]:
        fail("unexpected Tauri capability enabled")
    capability = json.loads(
        (ROOT / "src-tauri" / "capabilities" / "default.json").read_text("utf-8")
    )
    permissions = capability["permissions"]
    allowed = {"core:default", "dialog:allow-open", "dialog:allow-save"}
    simple_permissions = {item for item in permissions if isinstance(item, str)}
    scoped_permissions = [item for item in permissions if isinstance(item, dict)]
    expected_opener = {
        "identifier": "opener:allow-open-url",
        "allow": [
            {"url": "https://translate.google.com/*"},
            {"url": "https://translate.yandex.com/*"},
        ],
    }
    if simple_permissions != allowed or scoped_permissions != [expected_opener]:
        fail("desktop capability permissions changed without security review")
    scopes = set(security["assetProtocol"]["scope"])
    expected = {
        "$APPLOCALDATA/covers/**",
        "$APPLOCALDATA/fonts/**",
        "$APPLOCALDATA/readers/**",
    }
    if scopes != expected:
        fail("asset protocol scope changed without security review")

    bundle = config["bundle"]
    if bundle.get("active") is not True or bundle.get("targets") != ["nsis"]:
        fail("public Windows candidate must produce only the reviewed NSIS bundle")
    if bundle.get("useLocalToolsDir") is not True:
        fail("Windows bundler tools must remain in the project-local target cache")
    expected_book_associations = {
        "epub",
        "fb2",
        "txt",
        "html",
        "htm",
        "md",
        "markdown",
        "pdf",
        "cbz",
        "cbr",
        "docx",
    }
    expected_audio_associations = {
        "aac",
        "flac",
        "m4a",
        "m4b",
        "mp3",
        "wav",
        "wma",
        "3g2",
        "3gp",
        "amr",
        "aif",
        "aiff",
        "alac",
        "ape",
        "caf",
        "mka",
        "mpc",
        "oga",
        "ogg",
        "opus",
        "wv",
        "cue",
        "m3u",
        "m3u8",
    }
    associations = bundle.get("fileAssociations", [])
    if len(associations) != 2:
        fail("public Windows candidate must declare reviewed book and audio associations")
    book_association, audio_association = associations
    if set(book_association.get("ext", [])) != expected_book_associations:
        fail("Windows book file-association set changed without review")
    if set(audio_association.get("ext", [])) != expected_audio_associations:
        fail("Windows audio file-association set changed without review")
    if book_association.get("name") != "ApriReader Book":
        fail("Windows book association name changed without review")
    if audio_association.get("name") != "ApriReader Audiobook":
        fail("Windows audio association name changed without review")
    if any(association.get("role") != "Viewer" for association in associations):
        fail("ApriReader file associations must retain the Viewer role")
    if bundle.get("license") != "Apache-2.0" or bundle.get("licenseFile") != "../LICENSE":
        fail("installer license metadata changed without review")
    nsis = bundle.get("windows", {}).get("nsis", {})
    if nsis.get("installMode") != "currentUser":
        fail("installer must remain scoped to the current Windows user")
    if nsis.get("languages") != ["English", "Russian"]:
        fail("installer language set changed without review")
    if nsis.get("displayLanguageSelector") is not True:
        fail("installer must expose the reviewed RU/EN language selector")


def check_candidate_provenance() -> None:
    builder = (ROOT / "scripts" / "build_beta_candidate.ps1").read_text("utf-8")
    required_markers = {
        "sourceTreeState": "source tree state",
        "sourceChangedFileCount": "changed-file count",
        "SOURCE_SHA256SUMS.txt": "source manifest",
        "sourceManifestSha256": "source manifest hash",
        '"release-candidate", "github-release"': "public channel guard",
        "Release-candidate and GitHub release builds require -RequireCleanTree.": (
            "mandatory clean-tree public-build guard"
        ),
        "The release source tree changed while the candidate was building.": (
            "mid-build source mutation guard"
        ),
    }
    missing = [
        description
        for marker, description in required_markers.items()
        if marker not in builder
    ]
    if missing:
        fail(f"candidate provenance gate is stale: {', '.join(missing)}")
    if not (ROOT / "release" / "THIRD_PARTY_LICENSES.md").is_file():
        fail("generated third-party license bundle is missing")


def check_cloud_tts_boundary() -> None:
    native = (ROOT / "src-tauri" / "src" / "cloud_tts.rs").read_text("utf-8")
    ui = (ROOT / "src" / "ui" / "TextToSpeechPanel.tsx").read_text("utf-8")
    required_native = {
        '"https://api.elevenlabs.io/v2/voices"': "fixed voice endpoint",
        '"https://api.elevenlabs.io/v1/text-to-speech"': "fixed speech endpoint",
        'w!("ApriReader/ElevenLabsApiKey")': "Credential Manager target",
        "MAX_CLOUD_TEXT_CHARACTERS: usize = 2_000": "cloud text limit",
        "MAX_TTS_RESPONSE_BYTES: u64 = 48 * 1024 * 1024": "response limit",
        "MAX_CLOUD_AUDIO_BYTES: usize = 32 * 1024 * 1024": "audio limit",
        'header("xi-api-key", &key)': "native-only credential header",
        "alignment.characters.concat() != text": "exact alignment check",
    }
    missing = [
        description
        for marker, description in required_native.items()
        if marker not in native
    ]
    if missing:
        fail(f"cloud TTS boundary is stale: {', '.join(missing)}")
    if "api.elevenlabs.io" in ui:
        fail("ElevenLabs endpoint must not move into the WebView")
    if "writeLocalValue(apiKey" in ui or "writeLocalValue(cloudVoiceKey, apiKey" in ui:
        fail("cloud TTS API key must not enter WebView local storage")
    required_ui = {
        'readLocalValue(cloudConsentKey) === "accepted"': "stored consent state",
        't("ttsCloudConsentBody")': "first-send disclosure",
        "saveCloudTtsKey(apiKey)": "native credential handoff",
        "deleteCloudTtsKey()": "credential deletion action",
    }
    missing_ui = [
        description
        for marker, description in required_ui.items()
        if marker not in ui
    ]
    if missing_ui:
        fail(f"cloud TTS consent boundary is stale: {', '.join(missing_ui)}")


def check_tts_preferences_boundary() -> None:
    preferences = (
        ROOT / "src" / "application" / "ttsPreferences.ts"
    ).read_text("utf-8")
    ui = (ROOT / "src" / "ui" / "TextToSpeechPanel.tsx").read_text("utf-8")
    required_preferences = {
        "maxTtsVoicePresets = 20": "voice-preset limit",
        "maxTtsPronunciationRules = 100": "pronunciation-rule limit",
        "maxTtsSpokenChunkCharacters = 2_000": "post-replacement text limit",
        'ttsPreferencesStorageKey = "aprireader.tts.preferences.v1"': (
            "versioned local preference store"
        ),
        "hasWholeWordBoundaries": "whole-word matching guard",
        "sourceOffsets": "source-offset mapping",
        "remapCloudTtsTimings": "cloud timing remap",
        'throw new Error("TTS_PRONUNCIATION_EXPANSION_LIMIT")': (
            "replacement expansion guard"
        ),
    }
    missing = [
        description
        for marker, description in required_preferences.items()
        if marker not in preferences
    ]
    if missing:
        fail(f"TTS preference boundary is stale: {', '.join(missing)}")
    required_ui = {
        "applyPronunciationDictionary(": "synthesis-only dictionary application",
        "preferences.dictionaryEnabled": "dictionary opt-out",
        "normalizeVoicePreset({": "preset validation",
        "normalizePronunciationRule({": "rule validation",
        "remapCloudTtsTimings(": "source timing restoration",
    }
    missing_ui = [
        description
        for marker, description in required_ui.items()
        if marker not in ui
    ]
    if missing_ui:
        fail(f"TTS preference UI boundary is stale: {', '.join(missing_ui)}")
    if "apiKey" in preferences or "ElevenLabsApiKey" in preferences:
        fail("TTS local preferences must never include provider credentials")


def check_google_tts_boundary() -> None:
    native = (ROOT / "src-tauri" / "src" / "google_tts.rs").read_text("utf-8")
    ui = (ROOT / "src" / "ui" / "TextToSpeechPanel.tsx").read_text("utf-8")
    required_native = {
        '"https://texttospeech.googleapis.com/v1/voices"': "fixed voice endpoint",
        '"https://texttospeech.googleapis.com/v1/text:synthesize"': (
            "fixed synthesis endpoint"
        ),
        'w!("ApriReader/GoogleCloudTtsApiKey")': "Credential Manager target",
        "MAX_GOOGLE_TEXT_CHARACTERS: usize = 2_000": "character limit",
        "MAX_GOOGLE_TEXT_BYTES: usize = 4_800": "UTF-8 input limit",
        "MAX_VOICES_RESPONSE_BYTES: u64 = 4 * 1024 * 1024": (
            "voice response limit"
        ),
        "MAX_TTS_RESPONSE_BYTES: u64 = 48 * 1024 * 1024": (
            "speech response limit"
        ),
        "MAX_GOOGLE_AUDIO_BYTES: usize = 32 * 1024 * 1024": "audio limit",
        '.header("x-goog-api-key", &key)': "native-only credential header",
        '"audioEncoding": "MP3"': "reviewed output format",
    }
    missing = [
        description
        for marker, description in required_native.items()
        if marker not in native
    ]
    if missing:
        fail(f"Google TTS boundary is stale: {', '.join(missing)}")
    if "?key=" in native or '.query("key"' in native:
        fail("Google API key must never enter a request URL")
    if "texttospeech.googleapis.com" in ui:
        fail("Google Cloud endpoint must not move into the WebView")
    required_ui = {
        'readLocalValue(googleConsentKey) === "accepted"': (
            "separate stored consent state"
        ),
        't("ttsGoogleConsentBody")': "Google first-send disclosure",
        "saveGoogleTtsKey(googleApiKey)": "native credential handoff",
        "deleteGoogleTtsKey()": "credential deletion action",
        'void start("google")': "provider-specific consent continuation",
    }
    missing_ui = [
        description
        for marker, description in required_ui.items()
        if marker not in ui
    ]
    if missing_ui:
        fail(f"Google TTS consent boundary is stale: {', '.join(missing_ui)}")
    if "writeLocalValue(googleApiKey" in ui:
        fail("Google Cloud API key must not enter WebView local storage")


def check_azure_tts_boundary() -> None:
    native = (ROOT / "src-tauri" / "src" / "azure_tts.rs").read_text("utf-8")
    ui = (ROOT / "src" / "ui" / "TextToSpeechPanel.tsx").read_text("utf-8")
    required_native = {
        "const REGIONS: [(&str, &str); 33]": "reviewed region allowlist",
        'w!("ApriReader/AzureSpeechApiKey")': "Credential Manager target",
        '"https://{region}.tts.speech.microsoft.com/cognitiveservices/voices/list"': (
            "regional voice endpoint"
        ),
        '"https://{region}.tts.speech.microsoft.com/cognitiveservices/v1"': (
            "regional synthesis endpoint"
        ),
        'header("Ocp-Apim-Subscription-Key", &key)': "native credential header",
        'header("Content-Type", "application/ssml+xml")': "SSML content type",
        "MAX_AZURE_TEXT_CHARACTERS: usize = 2_000": "text limit",
        "MAX_AZURE_SSML_BYTES: usize = 16 * 1024": "SSML limit",
        "MAX_AZURE_AUDIO_BYTES: u64 = 32 * 1024 * 1024": "audio limit",
        '.replace(\'&\', "&amp;")': "XML escaping",
    }
    missing = [
        description
        for marker, description in required_native.items()
        if marker not in native
    ]
    if missing:
        fail(f"Azure TTS boundary is stale: {', '.join(missing)}")
    if "speech.microsoft.com" in ui:
        fail("Azure Speech endpoint must not move into the WebView")
    required_ui = {
        'readLocalValue(azureConsentKey) === "accepted"': "separate consent",
        't("ttsAzureConsentBody")': "Azure disclosure",
        "saveAzureTtsKey(azureApiKey)": "native credential handoff",
        "deleteAzureTtsKey()": "credential deletion",
        'void start("azure")': "provider-specific continuation",
    }
    missing_ui = [
        description
        for marker, description in required_ui.items()
        if marker not in ui
    ]
    if missing_ui:
        fail(f"Azure TTS consent boundary is stale: {', '.join(missing_ui)}")
    if "writeLocalValue(azureApiKey" in ui:
        fail("Azure Speech key must not enter WebView local storage")


def check_tts_assets_boundary() -> None:
    native = (ROOT / "src-tauri" / "src" / "tts_assets.rs").read_text("utf-8")
    ui = (ROOT / "src" / "ui" / "TextToSpeechPanel.tsx").read_text("utf-8")
    required_native = {
        "MAX_EXPORT_PARTS: usize = 5_000": "export part limit",
        "MAX_EXPORT_PART_BYTES: u64 = 64 * 1024 * 1024": "per-part export limit",
        "MAX_EXPORT_TOTAL_BYTES: u64 = 6 * 1024 * 1024 * 1024": (
            "aggregate export limit"
        ),
        "validate_cached_source": "cache containment validation",
        "classify_cache_name": "exact cache filename validation",
        'String::from("#EXTM3U\\n")': "local M3U8 export",
        "partial_directory": "partial export isolation",
        "sessions.len() >= 2": "concurrent export limit",
    }
    missing = [
        description
        for marker, description in required_native.items()
        if marker not in native
    ]
    if missing:
        fail(f"TTS cache/export boundary is stale: {', '.join(missing)}")
    required_ui = {
        "maxTtsExportParts": "frontend export limit",
        "appendTtsExportPart": "incremental cache-safe export",
        "ttsExportConfirmCloud": "cloud quota disclosure",
        "exportCancelled.current": "bounded cancellation state",
        "clearTtsCache(selectedProvider)": "provider cache deletion",
        'writeLocalValue(expressiveKey, JSON.stringify(next))': (
            "local expressive preference persistence"
        ),
    }
    missing_ui = [
        description
        for marker, description in required_ui.items()
        if marker not in ui
    ]
    if missing_ui:
        fail(f"TTS cache/export UI boundary is stale: {', '.join(missing_ui)}")
    if "remove_dir_all(cache_dir" in native:
        fail("TTS cache clearing must never recursively delete the cache root")


def main() -> int:
    check_forbidden_files()
    check_direct_dependencies()
    check_bundled_fonts()
    check_tauri_boundary()
    check_cloud_tts_boundary()
    check_tts_preferences_boundary()
    check_google_tts_boundary()
    check_azure_tts_boundary()
    check_tts_assets_boundary()
    check_candidate_provenance()
    print("Release security and direct-license gates passed.")
    for name, license_id in sorted(APPROVED_NPM.items() | APPROVED_CARGO.items()):
        print(f"  {name}: {license_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
