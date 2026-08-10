# Asset provenance

This register covers project-controlled visual and font assets used by the
application and distribution pages.

| Asset                                                                    | Origin and rights record                                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ApriReader application icon                                              | Original project vector in `src-tauri/icons/app-icon.svg`; raster and platform icon variants are derivatives of that source. It contains a generic open-book mark and no third-party logo.                                                               |
| Steam/library source key art                                             | Original project artwork drawn from the ApriReader open-book mark, page geometry, and the ivory, charcoal, brass, and walnut palette by `scripts/generate_steam_assets.py`.                                                                              |
| Steam/library capsules and logo                                          | Deterministic crops and project-name compositions generated from the original project artwork by `scripts/generate_steam_assets.py`; no review, award, discount, or third-party mark is embedded.                                                        |
| Literata, Lora, Merriweather, Source Serif 4, Charis SIL, IBM Plex Serif | Redistributed under SIL Open Font License 1.1. Complete license texts are included under `public/licenses/fonts` and in the generated third-party license bundle.                                                                                        |
| Application screenshots                                                  | Must be captured from the final build. Only synthetic fixtures, project-owned material, or verified public-domain books and covers may appear. Commercial book covers, copyrighted text, personal paths, API keys, and user library data are prohibited. |

Provider names such as Windows, Open Library, Inventaire, ElevenLabs, Google
Cloud, Yandex, and Azure are used only to identify interoperable services. Store
copy must not use provider logos or imply sponsorship, certification, or
partnership without separate written permission.

Before each marketplace submission, the publisher must verify that the exact
uploaded screenshots, trailer, capsule files, and descriptions are covered by
this register.
