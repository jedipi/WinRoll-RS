# GitHub Pages

The site is plain HTML and CSS with no build dependencies. Open `index.html` locally to preview it. It follows your system's light or dark appearance preference.

To publish, select **GitHub Actions** under the repository's **Settings → Pages → Build and deployment → Source**, then push these files to `main` or run **Deploy GitHub Pages** manually from the Actions tab. The expected project URL is https://jedipi.github.io/WinRoll-RS/.

Release buttons link to the repository's releases list; they do not assume a release has been published.

Run the markup and image asset check with `node site/check.cjs` from the repository root.

Preview through a local server with `python -m http.server 4173 --bind 127.0.0.1 --directory site`. The hero plays the 20-second HyperFrames promo from `assets/winroll-promo.mp4`, with native controls, a poster frame and English captions. It starts paused. The tray screenshot is supplied by the user. The two gesture illustrations were generated with the built-in image tool. UI icons are from Tabler Icons; their MIT license is included in `assets/icons/LICENSE`.
