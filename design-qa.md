# WinRoll RS landing page: Product Design QA

final result: passed

## Current hero video revision

The user's later request replaces the hero image with the updated 20-second promo. The page uses the local H.264/AAC MP4 and English WebVTT captions, with native controls and inline playback. The latest cover-frame request restores the original supplied desktop screenshot as the poster. `object-fit: contain` preserves its proportions within the 16:9 player. The video starts paused. Center alignment accommodates the media while preserving the existing text and mobile order. Asset checks pass for the cover revision; fresh visual verification was unavailable because the browser tool lost its tab session.

Browser verification confirms 1920 × 1080 video, duration 20 seconds, no media error, playback reaching the end at 20 seconds, and caption readyState 2 (loaded). At a 390px mobile viewport the player preserves 16:9 and no horizontal overflow occurs. Evidence: `target/site-qa/hero-video-desktop.jpg` and `target/site-qa/hero-video-mobile.jpg`. Local media, poster, caption and asset checks pass. The old site's unused hero PNG was removed; the original is preserved in the promo's capture assets.

## Current scoped annotation revision

The five browser comments supersede the original mock for the affected elements. Comments 1 and 4 refer to the same compatibility strip; that strip and Comment 5's recovery disclosure are removed, together with their unused script, CSS and icons. The drag illustration now uses a four-way move cross. The right-click cursor and click rays sit in the empty caption area, clear of the right-side buttons.

Combined before/after evidence: `target/site-qa/annotations-comparison.jpg`, with both captures normalized to 1122px-wide panels. The comparison was opened and inspected together. Focused live-page evidence: `target/site-qa/annotation-edits.jpg`. Revised desktop and mobile captures: `target/site-qa/annotations-desktop.jpg` and `target/site-qa/annotations-mobile.jpg`.

The comparison confirms unchanged typography, colors and composition outside the requested edits. The two revised raster assets were opened individually to inspect the cross and cursor placement. Browser checks confirm both removed blocks are absent, all four images load, no horizontal overflow at desktop or 390px mobile, and no console warnings/errors. `node site/check.cjs` and `git diff --check` pass. No remaining P0/P1/P2 issue was found for this scoped revision.

The following sections retain the original redesign's QA history. Their recovery-disclosure and compatibility-strip descriptions refer to the earlier revision, not the current page.

## Source and evidence

- Selected visual truth: option 3, revised with the user's tray screenshot, `C:/Users/jedi/.codex/generated_images/01a0f6cd-0b47-7b02-aa34-1f7daa7cc661/exec-70eb49f5-b103-48a3-a272-fde0631687cc.png`.
- Implementation: `http://127.0.0.1:4173/` in the Codex in-app browser.
- Source pixels: 1122 × 1402. Desktop CSS viewport: 1122 × 1402, device pixel ratio 1. Browser screenshot omits the scrollbar: 1107px wide. The comparison displays both inputs at 1122px wide, keeping aspect ratios and cropping only the implementation's additional below-reference content.
- Desktop implementation screenshot: `target/site-qa/desktop-final.jpg`.
- Full-view combined comparison: `target/site-qa/comparison-final.jpg`, source left and implementation right. This was opened and reviewed together, not inferred from code or separate image views.
- Mobile: `target/site-qa/mobile.jpg`, 390 × 844 CSS viewport, full-page capture.
- Tablet: `target/site-qa/tablet.jpg`, 768 × 1024 CSS viewport.
- State: light theme, top of page, recovery disclosure closed. The source only specifies light mode.

## Findings and accepted constraints

No remaining actionable P0/P1/P2 issues in the tested views.

- Typography: Segoe UI Variable/Segoe UI matches the reference's modern sans-serif character. Desktop hero and instruction headings use the intended two-line rhythm. Tablet wrapping was corrected and rechecked. The revised plain-language copy is deliberately shorter and excludes the mockup's broad compatibility promises.
- Layout: image-left hero, copy-right hierarchy, asymmetric gesture instructions, and the tinted tray block follow the selected direction. The actual screenshots have different internal window proportions from the image-generated mock; the implementation preserves the supplied originals rather than stretching or recreating them.
- Colors: near-white surface, charcoal type, rust accent, and a pale warm tray surface stay consistent. Calculated light and dark text/link/CTA contrast passes WCAG AA. Dark mode has token coverage but was not visually compared against a separate dark mock.
- Images: the user's desktop and tray screenshots are uncropped, preserve aspect ratios, and load successfully. The tray asset is byte-identical to the supplied PNG. Two generated gesture assets are placed in their instruction slots. Official Tabler assets supply the window mark, GitHub, arrows, document and Windows icons; none are handcrafted SVG approximations.
- Copy: real tray actions, roll/unroll behavior and retained recovery limitations match repository documentation. Release availability is not invented. Build instructions and recovery details remain below the selected mock's content for continuity with the existing page.
- Focused review: the hero headline, gesture captions, tray screenshot, control labels and explanatory text were reviewed within the combined full-resolution comparison. They are readable there; a separate clipped capture was unreliable and is not used as acceptance evidence.

## Comparison history

1. Initial combined evidence: `target/site-qa/comparison-before.jpg`. P2: hero and instruction typography were too small and the hero's copy started too high. Adjusted grid proportions, headline sizes and hero copy positioning. P2: tray section was too tall. Shortened explanations and reduced row spacing. Post-fix comparison: `target/site-qa/comparison-after.jpg` and final combined evidence above.
2. Mobile check found 2px of overflow from the build grid's implicit minimum width. Changed the mobile track to `minmax(0, 1fr)`. Recaptured `mobile.jpg`; document width no longer exceeds the viewport.
3. Tablet check found a three-line hero heading at 768px. Changed the intermediate font size to a clamp. Recaptured `tablet.jpg`: heading height 96.78px at 48.4px line-height, two lines. No horizontal overflow.

## Interaction and technical checks

- How it works and Get started links navigate to their retained anchors.
- Compatibility and recovery link opens the native disclosure and navigates to it; its summary also closes it correctly.
- View releases points to the repository's GitHub releases page; the usage guide and issue link retain their real destinations.
- All four raster assets load in the browser. Local CSS icon assets and accessible heading references pass `node site/check.cjs`.
- Browser console error/warning log was empty during interaction checks.
- Checked 320, 390, 768, 1024 and 1440px widths for horizontal overflow; corrected the mobile and tablet issues noted above.
- `git diff --check` passes. No application code changed.

## Follow-up polish

- P3: generated cursor/title-bar proportions differ slightly from the mock's illustrations; they remain recognizable supporting imagery.
- P3: icon-library mark and GitHub icon use slightly different stroke shapes from the generated mock.
- Not measured: Lighthouse performance scores and visual dark-mode QA. Neither is represented as a completed check.

## Implementation checklist

- Selected layout and real screenshots implemented.
- Browser interaction and responsive checks completed.
- Actionable visual findings corrected and recaptured.
- Local preview left running; no publication or deployment performed.
