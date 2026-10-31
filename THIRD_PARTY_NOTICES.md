# Third-party notices

The root MIT license applies to original Nocan project code. It does not replace the licenses of Rust dependencies, grant rights to third-party trademarks, or license content supplied by external services.

## Rust dependencies

Direct dependencies are `windows`, `serde`, `serde_json`, and `ureq`. Their transitive dependencies also retain their respective licenses and copyright notices. `Cargo.lock` records the resolved versions.

To inspect the dependency manifests and license declarations for a checkout:

```powershell
cargo metadata --locked --offline --format-version 1
```

Cargo's downloaded registry sources contain the license files associated with each dependency. Include the applicable dependency notices when distributing compiled software; the root license alone is not a replacement for those notices.

## Website animation libraries

The landing page includes GSAP and ScrollTrigger 3.15.0 under `docs/assets/vendor`. These libraries are covered by the GSAP Standard License, not the root MIT license. Their original license headers are preserved. See `docs/assets/vendor/GSAP-NOTICE.txt` and https://gsap.com/standard-license.

## Provider marks and external content

The icons under `assets/logos`, and their website copies under `docs/assets/providers`, identify Gemini, Claude, Codex, and OpenCode integrations. Their names and marks belong to their respective owners. They do not indicate affiliation or endorsement, and the project's MIT license does not grant trademark rights.

Lyrics, album artwork, media metadata, stock-market data, and externally hosted source icons are supplied by their respective services or applications. Their terms and rights remain separate from Nocan's source-code license.

The website's `docs/assets/bleachers-merry-christmas.jpg` identifies Bleachers' single _Merry Christmas, Please Don't Call_ (℗ 2024 Dirty Hit). The artwork and metadata were obtained from the [Apple Music catalog](https://music.apple.com/us/album/merry-christmas-please-dont-call/1775877095?i=1775877096) for the requested music-interface example. Artwork and the short lyric excerpt retain their original rights and are not covered by the project's MIT license. No song audio or full lyrics are bundled in this repository.

The browser preview requests time-synced lyrics from [LRCLIB](https://lrclib.net/docs) at runtime. That external content retains its original rights and is not licensed under this repository's MIT license. Responses are not committed to the repository or stored in browser persistence.

`docs/assets/lake-wallpaper.jpg` is a reduced-resolution copy of [A view of a mountain lake surrounded by pine trees](https://unsplash.com/photos/a-view-of-a-mountain-lake-surrounded-by-pine-trees-WPSbXAoiOc0) by Sofia, used under the [Unsplash License](https://unsplash.com/license). The photo retains its original license and is not covered by the project's MIT license.

Nocan draws its desktop interface with Windows APIs and uses installed fonts. No Apple system framework or font is bundled. The Nocan tiger runtime icon is included under `assets/mascot/runtime`; the mascot documentation describes the included artwork and renderer.
