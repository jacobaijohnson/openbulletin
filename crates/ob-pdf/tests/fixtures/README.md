# Variable-font test fixtures (R51-D)

The Church Starter Pack's three variable families, as the app's own
catalog installer fetches them: single-file variable fonts from the
google/fonts repository (OFL-1.1), downloaded 2026-11-01 for the
round-51 G-04 fix. Each file carries one `wght` axis (default 400);
Gelasio and Lora span 400-700, Playfair Display 400-900. All four
are TrueType-glyf outlines, which is what the PDF writer's
wght-instantiation path supports.

| File | SHA-256 | Source |
| --- | --- | --- |
| `Gelasio[wght].ttf` | `4daecea457258c9ebeb8bc99ed3fd24353618bfad3ea4b93fa0b5d0468fc04e4` | `https://raw.githubusercontent.com/google/fonts/main/ofl/gelasio/Gelasio[wght].ttf` |
| `Gelasio-Italic[wght].ttf` | `52559e845a4d33514e5f93bb9ae7dbeae1894a53f2c565a15f18af40cd337c09` | `https://raw.githubusercontent.com/google/fonts/main/ofl/gelasio/Gelasio-Italic[wght].ttf` |
| `Lora[wght].ttf` | `822a6621ccbe8d97d20ac88c1c41f5615c9c2c202eaa75f272cd452aac6475a7` | `https://raw.githubusercontent.com/google/fonts/main/ofl/lora/Lora[wght].ttf` |
| `PlayfairDisplay[wght].ttf` | `c40f2293766a503bc70cce9e512ef844a4ccb7cbcde792fe2ea31d191917d8d6` | `https://raw.githubusercontent.com/google/fonts/main/ofl/playfairdisplay/PlayfairDisplay[wght].ttf` |

Test-only fixtures: they are never embedded in the app or shipped
in a bundle. The library never installs them; tests build library
snapshots from these bytes directly.
