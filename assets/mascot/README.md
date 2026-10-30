# Nocan mascot and branding assets

Nocan combines Notch and Macan, the Indonesian word for tiger. The runtime application icon and animated desktop face use the Nocan tiger identity.

## Runtime assets

| File | Purpose |
| --- | --- |
| `runtime/nocan.png` | Nocan tiger application icon |
| `runtime/nocan.ico` | Embedded Windows application, tray, and window icon |
| `runtime/nocan-64.bgra` | Raw 64 × 64 icon data available to the renderer |

The icon contains 16, 20, 24, 32, 40, 48, 64, 128, and 256 pixel variants. `build.rs` compiles the icon and Windows manifest into the executable, along with Nocan product/version metadata.

The runtime assets are committed and ready for the Rust build. Update the PNG, ICO variants, and raw icon data together when changing the application icon.

## Concept artwork

The three `nocan-tiger-*-v1.png` illustrations show the full-body, head-only, and peeking concepts. These were created with an image-generation tool; their generation prompts are stored under `prompts/`. They document the character's development and are not animation sprite sheets.

The production icon is included under `runtime/`. The live desktop mascot is drawn directly in Rust and Direct2D in `src/notch/mascot.rs`; it does not depend on the concept PNGs.

## Live animation

The mascot has white forehead stripes, eyes, and whiskers over a dark surface. Its pupils follow the pointer, it blinks automatically, and clicking or pressing Enter/Space triggers a reaction. Music playback adds gentle motion; the animation does not measure the audio waveform.

`src/graphics/ambient.rs` provides the shared cyan, blue, and violet ambience. A circular field of dots remains circular as the notch width changes. All drawing is clipped to the notch silhouette.

The normal mascot view is 188 × 40 pixels, and the compact clock view is 188 × 32. The standard expanded panel is 448 × 196; Tasks and OpenMaus use wider/taller layouts, while lyrics resize to fit their content. Minimal mode and the floating timer have their own size rules.

## Preview generation

```powershell
cargo build --locked
.\target\debug\nocan.exe --preview-branding target\branding-preview
```

The native preview exports 100 raw mascot frames and section snapshots, then checks native click and keyboard actions. Generated review assets are excluded from version control.

Preview commands automatically isolate core settings, tasks, and water history in a temporary profile, unless `NOCAN_DATA_DIR` was explicitly set. Other integrations can still read their own local data; see the root README for the profile override's scope.

## Provider icons

The provider marks under `assets/logos` belong to their respective owners and identify integrations. Their rights are separate from the original Nocan artwork and code. See [third-party notices](../../THIRD_PARTY_NOTICES.md).
