# Distribution claim matrix

Use this matrix when preparing GitHub, itch.io, FOSSHub, WinGet, Chocolatey,
Steam, or catalog listings. Claims not supported by the selected package must
not appear on its page.

| Claim                                               | NSIS installer                                       | Portable/Depot build                                 | Required wording                                                                                                                        |
| --------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Windows 10/11 x64                                   | Yes, subject to final clean-system verification      | Yes, subject to final clean-system verification      | Do not claim other operating systems.                                                                                                   |
| EPUB, PDF, FB2, TXT, HTML, Markdown, CBZ, CBR, DOCX | Yes                                                  | Yes                                                  | DRM-free local files only.                                                                                                              |
| Audiobook playback                                  | Yes                                                  | Yes                                                  | Uses the Windows media stack; system-codec-tier formats require a compatible decoder installed in Windows. DRM formats are unsupported. |
| Explorer file associations                          | Installed by the ApriReader NSIS installer           | Not registered automatically                         | Mention only for the installer package.                                                                                                 |
| Local Windows read-aloud                            | Yes                                                  | Yes                                                  | Uses voices already installed in Windows.                                                                                               |
| Cloud voices                                        | Optional                                             | Optional                                             | BYOK, provider account/network/terms/quota may apply, explicit consent required. Never call them bundled or free voices.                |
| Metadata and translation                            | Optional network action                              | Optional network action                              | Name the providers and state that requests occur only after user action.                                                                |
| Achievements                                        | 42 local achievements                                | 42 local achievements                                | Do not select the Steam Achievements feature until the protected App ID build passes its separate checklist.                            |
| Privacy                                             | Local-first, no telemetry or ads                     | Local-first, no telemetry or ads                     | Do not claim the application is completely offline because optional provider actions use the network.                                   |
| Updates                                             | Manual download unless a platform manages updates    | Managed only when the platform supplies that service | Do not claim an in-app updater.                                                                                                         |
| Code signing                                        | Only when Authenticode status is verified as `Valid` | Only when Authenticode status is verified as `Valid` | Current unsigned artifacts must be labelled as unsigned and may trigger Windows warnings.                                               |

The canonical short description is:

> ApriReader is a free, open-source, local-first Windows reader and personal
> library for DRM-free books and audiobooks.

The Steam build may use the same description, but its store feature selections
must omit Steam Achievements and automatic Explorer associations until those
features are proven in the submitted Steam package.
