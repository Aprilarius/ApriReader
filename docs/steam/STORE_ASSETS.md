# Steam store assets

Generated Stage 8 capsule sources live in `docs/steam/assets`. They use only the
ApriReader name, original key art, and the approved ivory, charcoal, brass, and
walnut palette.

| File                  |        Size | Purpose                         |
| --------------------- | ----------: | ------------------------------- |
| `store-header.png`    |   920 x 430 | Required header capsule         |
| `store-small.png`     |   462 x 174 | Required small capsule          |
| `store-main.png`      |  1232 x 706 | Required main capsule           |
| `store-vertical.png`  |   748 x 896 | Required vertical capsule       |
| `library-capsule.png` |   600 x 900 | Required library capsule        |
| `library-header.png`  |   920 x 430 | Required library header         |
| `library-hero.png`    | 3840 x 1240 | Required text-free library hero |
| `library-logo.png`    |  1280 x 360 | Transparent library logo        |
| `page-background.png` |  1438 x 810 | Optional store background       |

Capsules contain no reviews, awards, discounts, feature claims, third-party
marks, or text other than the product name. The hero and page background
contain no text. Before upload, review every asset in the current Steamworks
template and capture at least five real 1920 x 1080 product screenshots.

The original key art was generated with the built-in OpenAI image generator.
`scripts/generate_steam_assets.py` performs deterministic crops, logo placement,
and size validation.
