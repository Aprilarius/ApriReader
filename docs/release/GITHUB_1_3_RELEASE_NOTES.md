# ApriReader 1.3.0 RC1

This release candidate consolidates audiobook stages A0-A13 and the final
player, light-theme, and narration-queue fixes into one reviewable build.

ApriReader 1.3 adds a complete audiobook and read-aloud experience while
keeping the existing Windows book library, readers, annotations, statistics,
and local-first privacy model intact.

## Highlights

- Dedicated audiobook library with single-file and multi-part import, watched
  folders, natural ordering, source reconnection, search, metadata, covers,
  listening statistics, and separate achievements.
- Full audiobook player with resume, queue, chapters, bookmarks, sleep timer,
  speed and volume controls, output-device selection, automatic part advance,
  background playback, Windows media controls, and configurable tray behavior.
- Safe local CUE, M3U, and M3U8 support plus Explorer associations for the
  reviewed audiobook and playlist formats.
- Local Windows read-aloud for reflow books with installed voices, continuous
  section or whole-book narration, background preparation, active-word focus,
  automatic scrolling, pronunciation rules, and reusable voice presets.
- Optional bring-your-own-key ElevenLabs, Google Cloud Text-to-Speech, and
  Azure AI Speech providers with separate consent, protected Windows
  Credential Manager storage, bounded responses, provider-specific controls,
  and independent caches.
- Incremental M3U8 audiobook export from generated narration with bounded file,
  fragment, and package limits.
- Responsive audiobook-player layout, corrected light-theme action contrast,
  and reliable continuation from short chapter titles into the main text.

## Privacy and installation

Local Windows narration does not send book text to a server. Cloud voices are
strictly optional and require the user's own provider key plus explicit consent
before text is transmitted. Provider keys remain in Windows Credential Manager
and are never returned to the WebView after saving.

ApriReader does not modify source books or audio files. Generated speech,
covers, indexes, progress, bookmarks, and caches stay in application-managed
local storage.

The Windows installer is currently unsigned and may show an unknown-publisher
warning. Treat RC1 as a pre-release validation build, obtain it only from the
official candidate package, and verify its included SHA-256 checksum.
