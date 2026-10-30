# Nocan landing page

The landing page is a static website in `docs/index.html`, with styles and behavior under `docs/assets`. It is published from this repository through GitHub Pages. The intended custom domain is `nocan.fph.my.id`.

## Design and interactions

The page follows Nocan's light surfaces, neutral typography, blue accent, and tiger identity. The type uses the visitor's system fonts. The desktop illustration is drawn in SVG and CSS, while the Settings images are actual application renders.

The notch-shaped interface is the repeated visual motif. Its dark surface belongs to the product demonstration; the surrounding light page keeps text and controls readable. Blue highlights the main headline and active controls. The wallpaper's blue layers establish the Windows desktop setting rather than decorating unrelated content.

The layout varies between a centered introduction, a scroll narrative, an interactive module selector, an actual Settings screenshot, and source/install information. Motion introduces the notch, explains its timer/lyrics relationship, and confirms actions. The presentation uses moderate energy, varied composition, and purposeful animation (energy 2/5, rhythm 3/5, motion 3/5).

The browser previews use illustrative data, are labeled as previews, and do not access the visitor's desktop applications. They support timer start/pause, reset, mode changes, adding five minutes, the final-minute state, music controls without audio, lyrics, water logging, reminder and goal changes, and task creation, editing, completion, deletion, and pagination. Tasks entered in the browser demo remain in memory until the page is reloaded. OpenMaus tabs, provider selectors, stock selectors, and the notch's bottom navigation also work with example data.

`assets/notch-preview.js` and `assets/notch-preview.css` reproduce the native geometry from `src/sections/deck.rs`, `src/notch/window.rs`, `src/notch/mascot.rs`, and `src/notch/focus_capsule.rs`. The base panel is 448 × 196, Tasks is 520 × 232, OpenMaus is 560 × 232, and the lyrics panel is 560 × 196. Header positions, 32 px bottom corners, bottom navigation, focus ring, action controls, typography sizes, and the ambient gradient follow the application renderer. The compact mascot uses the native 188 × 40 layout, dot field, forehead stripes, eyes, and whiskers. The browser scales the full coordinate system together when space is limited; font rendering can differ between browsers and Windows DirectWrite.

The timer chip uses the native 164 × 36 proportions and green-to-amber-to-red progression. It joins the compact notch during normal use and detaches beside active lyrics. Opening the full notch hides the detached chip, as in the desktop app.

GSAP morphs the notch silhouette between its measured bounds when opening, collapsing, or changing section. Content fades after the shape starts moving, and the lyric timer enters separately. Native concave corners attach to the screen edge. Preview scaling uses layout width rather than transformed bounds, so morph animations do not change the final scale.

The music example is Bleachers' [Merry Christmas, Please Don't Call](https://music.apple.com/us/album/merry-christmas-please-dont-call/1775877095?i=1775877096), released in 2024, with a duration of 3:22. Metadata and the single artwork come from Apple's public catalog. The preview contains a short refrain only; no audio or complete lyrics are bundled. Playback controls move an illustrative progress clock.

ScrollTrigger pins the desktop narrative, changes Focus/Music/Water scenes, and scrubs the progress track. Smaller screens keep the illustration sticky above the chapters while their triggers remain active. The motion toggle and operating-system reduced-motion preference remain respected. When verifying the page, restore the motion setting instead of leaving an interactive preview paused after a check.

GSAP and ScrollTrigger 3.15.0 are bundled under `assets/vendor`. Their copyright notices are retained, with license details in `assets/vendor/GSAP-NOTICE.txt`. No package installation is needed to serve the production page.

The page respects reduced-motion preferences and offers a persistent motion toggle. Desktop scrolling pins the feature illustration; smaller viewports use a flowing layout. Module tabs support arrow keys, Home, and End. Interactive controls have focus indicators and mobile touch targets.

## Releases and privacy

The install button initially links to the source build instructions. The page checks GitHub's public releases API for a stable release with a Windows executable. When one is available, it uses the published download URL. Unavailable or rate-limited API responses leave the source build link functional.

The site stores only its motion preference in browser storage. The optional clipboard action copies the public clone command. All visual assets and animation libraries are served with the page. The release check is the only application-initiated external request; GitHub Pages itself has its own hosting behavior.

## GitHub Pages and domain

Use branch `master`, source directory `/docs`, and the committed `.nojekyll` file for static publication.

For the custom domain, set `nocan.fph.my.id` in the repository's Pages settings, then add the following record in the DNS zone for `fph.my.id`:

| Field  | Value              |
| ------ | ------------------ |
| Type   | CNAME              |
| Name   | nocan              |
| Target | fphaikal.github.io |
| TTL    | Auto               |

Use DNS-only mode while verifying the domain and issuing the certificate if the DNS provider offers proxying. Enable Enforce HTTPS in GitHub Pages after certificate issuance. Configure the Pages custom domain before adding the DNS record.

Until DNS and the custom domain are ready, the default Pages URL remains available at `https://fphaikal.github.io/Nocan/`. Documentation and application source remain in the same repository.
