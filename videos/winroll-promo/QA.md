# Promo verification

## GitHub Pages hero export

The user's hero-video request authorizes MP4 export. The updated composition was rendered to `renders/winroll-promo.mp4` and copied to `site/assets/winroll-promo.mp4`. FFprobe confirms H.264 video at 1920 × 1080, 30fps, AAC narration, and exactly 20.000000s. Size: 4,570,651 bytes. Render completed with hardware GPU screenshot capture in 32.9s. Export frames were compared with the source using `hyperframes snapshot --against`.

The page uses native video controls, inline playback, and English WebVTT captions. The latest cover request uses the original desktop screenshot as its poster, with `object-fit: contain` to preserve proportions. The MP4 itself is unchanged. The assets use relative paths suitable for the GitHub Pages project URL. The video is paused by default. Local asset checks pass and desktop browser playback has been tested.

## Supplied roll/unroll screenshot revision

Beats 2 and 3 now use the user's expanded and rolled File Explorer screenshots, copied unchanged. Beat 2 shows Expanded to Rolled up. Beat 3 moves the rolled window, then shows Unrolled at that same moved position. The images are aligned at the top left and retain their intrinsic proportions. Scene timing, voiceover, other scenes and the 20-second root duration are preserved.

`npm run check` passes with zero errors/warnings; 31 text checks pass WCAG AA. The information-only overlaps still correspond to the existing scene crossfades. All four status frames were opened and reviewed: `snapshots/frame-00-at-5.4s.png`, `frame-01-at-6.7s.png`, `frame-02-at-8.1s.png`, and `frame-03-at-9.4s.png`. Each actual screenshot and state label is visible, with no cropping, stretching or text collision.

The remaining entries describe the original promo's verification history.

- Duration: exactly 20 seconds; 1920 × 1080 landscape.
- Voiceover: 16.213375s, imported measured word onsets at 4.30, 7.24 and 9.98s for scene boundaries. Final spoken word ends at 14.88s; final card holds to 20s.
- `hyperframes lint`: zero errors/warnings.
- `hyperframes validate`: no console errors, 30 text checks pass WCAG AA.
- `npm run check`: zero errors/warnings; six information-only text overlaps occur during the intentional crossfades.
- `hyperframes inspect`: zero layout issues at 2.7, 6, 8.8, 12.5, 16 and 19.5s.
- Every scene and the final hold were visually reviewed from HyperFrames snapshots. The movement headline was reduced from 110px to 84px and given a wider text zone to fit two readable phrases. Its subtitle now finishes entering earlier. Updated keyframes were recaptured and reviewed.
- Native `hyperframes keyframes` maps 45 tweens across the root and four scenes, with no unresolved targets. Diagnostics are stored in `.hyperframes/animation-map.json`.
- Source palette, actual screenshots, empty-caption pointer placement and four-way cross are preserved. No fabricated benchmark, compatibility or download availability claim appears.
- Persistent Studio preview: `http://localhost:3002/#project/winroll-promo`, managed by `hyperframes preview --background`; verified with `preview --status`.
- Studio was opened and scrubbed to every beat. The Play control starts playback, the visible scene changes with the playhead, and the timeline reports 20 seconds. Studio lint also reports all checks passed after the retained raw website source was renamed to `.txt` to avoid being mistaken for a composition.
- No MP4 export was run. The HyperFrames workflow delivers the editable Studio preview until the user requests export.
