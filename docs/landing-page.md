# Nocan landing page

The landing page is a static website in `docs/index.html`, with styles and behavior under `docs/assets`. It is published from this repository through GitHub Pages. The intended custom domain is `nocan.fph.my.id`.

## Design and interactions

The page follows Nocan's light surfaces, neutral typography, blue accent, and tiger identity. The type uses the visitor's system fonts. The desktop illustration is drawn in SVG and CSS, while the Settings images are actual application renders.

The notch-shaped interface is the repeated visual motif. Its dark surface belongs to the product demonstration; the surrounding light page keeps text and controls readable. Blue highlights the main headline and active controls. The wallpaper's blue layers establish the Windows desktop setting rather than decorating unrelated content.

The layout varies between a centered introduction, a scroll narrative, an interactive module selector, an actual Settings screenshot, and source/install information. Motion introduces the notch, explains its timer/lyrics relationship, and confirms actions. The presentation uses moderate energy, varied composition, and purposeful animation (energy 2/5, rhythm 3/5, motion 3/5).

The browser previews use illustrative data, are labeled as previews, and do not access the visitor's desktop applications. They support timer start/pause, the final-minute state, music preview controls without audio, water logging, and task completion.

GSAP and ScrollTrigger 3.15.0 are bundled under `assets/vendor`. Their copyright notices are retained, with license details in `assets/vendor/GSAP-NOTICE.txt`. No package installation is needed to serve the production page.

The page respects reduced-motion preferences and offers a persistent motion toggle. Desktop scrolling pins the feature illustration; smaller viewports use a flowing layout. Module tabs support arrow keys, Home, and End. Interactive controls have focus indicators and mobile touch targets.

## Releases and privacy

The install button initially links to the source build instructions. The page checks GitHub's public releases API for a stable release with a Windows executable. When one is available, it uses the published download URL. Unavailable or rate-limited API responses leave the source build link functional.

The site stores only its motion preference in browser storage. The optional clipboard action copies the public clone command. All visual assets and animation libraries are served with the page. The release check is the only application-initiated external request; GitHub Pages itself has its own hosting behavior.

## GitHub Pages and domain

Use branch `master`, source directory `/docs`, and the committed `.nojekyll` file for static publication.

For the custom domain, set `nocan.fph.my.id` in the repository's Pages settings, then add the following record in the DNS zone for `fph.my.id`:

| Field | Value |
| --- | --- |
| Type | CNAME |
| Name | nocan |
| Target | fphaikal.github.io |
| TTL | Auto |

Use DNS-only mode while verifying the domain and issuing the certificate if the DNS provider offers proxying. Enable Enforce HTTPS in GitHub Pages after certificate issuance. Configure the Pages custom domain before adding the DNS record.

Until DNS and the custom domain are ready, the default Pages URL remains available at `https://fphaikal.github.io/Nocan/`. Documentation and application source remain in the same repository.
