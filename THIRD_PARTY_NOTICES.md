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

The icons under `assets/logos` identify Gemini, Claude, Codex, and OpenCode integrations. Their names and marks belong to their respective owners. They do not indicate affiliation or endorsement, and the project's MIT license does not grant trademark rights.

Lyrics, album artwork, media metadata, stock-market data, and externally hosted source icons are supplied by their respective services or applications. Their terms and rights remain separate from Nocan's source-code license.

Nocan draws its desktop interface with Windows APIs and uses installed fonts. No Apple system framework or font is bundled. The Nocan tiger runtime icon is included under `assets/mascot/runtime`; the mascot documentation describes the included artwork and renderer.
