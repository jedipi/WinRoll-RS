# WinRoll RS: Keep it open. Make room.

**Format:** 1920 × 1080, exactly 20 seconds.
**Audio:** local Kokoro narration, conversational and clear. No generic music added.
**Style basis:** DESIGN.md and the current WinRoll RS landing page.
**Timing:** measured narration word onsets: 0, 4.30, 7.24, 9.98 seconds. Narration length 16.213375s; final spoken word ends at 14.88s. Hold the final card through 20s. Crossfades overlap scenes by 0.45s.

## Asset audit
| Asset | Beat | Role |
|---|---|---|
| winroll-desktop.png | 1, 3, 4 | Actual product workspace, full uncropped image |
| right-click-v2.png | 2 | Right-click in empty caption, clear of controls |
| left-drag-v2.png | 3 | Four-way cross and rolled-window movement |
| winroll-tray.png | 4 | Actual menu, preserved at intrinsic aspect ratio |
| app-window.svg | 1, 2, 3, 4 | Brand mark in persistent small header |
| brand-github.svg | 4 | Releases CTA |
| arrow-right.svg | 4 | CTA direction |
| arrow-down.svg | SKIP | Not relevant to the promo |

## Global rules
One main message per scene; supporting imagery has large presence.
Use authentic screenshots with no clipping or invented application controls.
Layers: tinted or ghost-text background, main screenshot/gesture, foreground type and brand.
Gentle CSS crossfades handle all three transitions; no pre-transition content exits.
Entrances use different directions, opacity and scale, with three easing families per beat.
Keep narrative motion visible: slide to move, scale push for product image, kinetic phrase entry.
No auto-loop, random values or asynchronous timeline construction.

## Beat 1: Make room (0-4.75s; transition begins 4.30s)
**Concept:** A desktop with useful apps still open. The window screenshot leads; the headline makes the benefit immediate.
**VO:** Keep it open. Make room. This is WinRoll R S.
**Layout:** full-frame scene-content padded 100px 120px. Small brand header. Main 2-column row: screenshot at left 960px wide, heading and supporting sentence at right 620px wide.
**Text:** 'Keep it open.' / 'Make room.' as distinct block spans; secondary 'A smaller footprint. Same workspace.'
**Assets:** ../capture/assets/winroll-desktop.png, ../capture/assets/app-window.svg.
**Motion:** logo settles from scale .92; headline phrases slide from opposite horizontal offsets; screenshot enters with perspective rotationY -5 and x -50, then slowly scales 1 to 1.025. Supporting line rises. Use power3.out, expo.out and sine.out.
**Depth:** pale tint panel behind screenshot, small brand header in foreground. No cropped real screenshot.
**Transition:** root crossfade into beat 2, 0.45s. Keep outgoing content intact.
**SFX:** none; narration carries the hook.

## Beat 2: Right-click (4.30-7.69s; transition begins 7.24s)
**Concept:** Show the simple gesture clearly without pretending to record an actual application session.
**VO:** Right-click an empty title bar to roll up.
**Layout:** full-frame scene-content padded 100px 120px. Small brand header; text block at left 720px wide; the existing right-click-v2.png at right 860px wide, preserving image ratio. Main row vertically centered. No extra fake windows.
**Text:** 'Right-click.' / 'Roll up.' Supporting 'Use an empty caption area.'
**Assets:** ../capture/assets/right-click-v2.png, ../capture/assets/app-window.svg.
**Motion:** heading phrases cascade from x -55 then y 45; illustration enters with scale .86, opacity and a slight rotation, then a small scale pulse once as caption message lands. Body fades in separately. Thin rust rule expands scaleX from left. Use back.out(1.15), power2.out and sine.out.
**Depth:** pale-tint image region and foreground gesture pointer; type remains on clear surface.
**Transition:** crossfade into beat 3, 0.45s, no exits.

## Beat 3: Move it (7.24-10.43s; transition begins 9.98s)
**Concept:** The rolled title bar moves aside while the original workspace stays open. Movement is the point, not decorative choreography.
**VO:** Drag to move. Unroll when you need it.
**Layout:** full-frame scene-content padded 100px 120px. Header brand. Main row: text at left 640px wide, visual zone at right 900px wide. Use left-drag-v2.png large (860px) above a smaller actual desktop screenshot (680px), vertical gap 20px; preserve both aspect ratios and keep total height within 720px.
**Text:** 'Drag to move.' / accent 'Unroll when ready.' Supporting 'Your apps stay open.'
**Assets:** ../capture/assets/left-drag-v2.png, ../capture/assets/winroll-desktop.png, ../capture/assets/app-window.svg.
**Motion:** illustration translates smoothly 80px right then settles, matching four-way move cross; screenshot enters from scale .96 then gently pans x -10. Display phrases rise with short stagger, supporting line enters from x -25. Use power3.out, sine.inOut and expo.out.
**Depth:** actual desktop behind/under the gesture, text and brand foreground. No overlapping text or clipping.
**Transition:** crossfade into beat 4, 0.45s.

## Beat 4: Tray and call to action (9.98-20s)
**Concept:** A small real menu gives control. Finish on the product name and a practical GitHub action.
**VO:** Control from the tray. WinRoll R S. View releases on GitHub.
**Layout:** full-frame scene-content padded 100px 120px. Header brand. Main row: actual tray screenshot 580px wide on a pale-tint zone left; right text 820px wide with headline, concise action names, charcoal CTA and repository address. Header logo remains visible. No fake UI buttons inside the screenshot.
**Text:** 'Control from the tray.' Supporting 'Pause. Unroll all. Exit.' CTA 'View releases on GitHub'. URL 'github.com/jedipi/WinRoll-RS'.
**Assets:** ../capture/assets/winroll-tray.png, ../capture/assets/app-window.svg, ../capture/assets/brand-github.svg, ../capture/assets/arrow-right.svg.
**Motion:** screenshot slides from x -35 and scale .95, then drifts y -8; title rises; action names stagger; CTA settles from scale .94; URL fades in. Different eases: power2.out, expo.out and sine.out. Hold the CTA to the 20-second end; no final disappearing frame.
**Depth:** tinted menu region, real menu shadows, forward CTA in foreground.
**SFX:** none. Leave clean reading time after the last spoken word.

## Production structure
- index.html: 20-second root, narration and crossfade orchestration.
- compositions/beat-1.html through beat-4.html: four registered paused timelines.
- capture/: source screenshots, exact restored assets and tokens.
- DESIGN.md, SCRIPT.md, narration.txt: brand and copy references.
- narration.wav and transcript.json: actual voice and timing.
- snapshots/: reviewed keyframes; Studio preview is the handoff.

## Supplied Explorer status update

The user's later screenshots supersede the gesture illustration assignments in beats 2 and 3. Both use `capture/assets/explorer-expanded.png` and `capture/assets/explorer-rolled.png`, copied unchanged. Beat 2 starts expanded, then crossfades to rolled at local 1.65s. Beat 3 starts rolled, moves 40px to the right, then crossfades to expanded at local 1.22s (global 8.46s), matching the spoken unroll cue. The state labels follow the same transitions. Both image states share an aligned upper-left anchor and intrinsic aspect ratio. Other scenes, scene boundaries, voiceover and the 20-second root duration are unchanged.

