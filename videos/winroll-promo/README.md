# WinRoll RS: 20-second product promo

1920 × 1080 landscape composition with four scenes, 0.45-second crossfades, local narration, the user's real screenshots, and a GitHub release call to action. The root duration is exactly 20 seconds. The roll and unroll states use the supplied expanded and rolled File Explorer screenshots, preserving their aspect ratios.

Preview in HyperFrames Studio:

```powershell
npx --yes hyperframes@0.8.107 preview --background --no-open --port 3002
```

Open `http://localhost:3002/#project/winroll-promo`. The studio is the review surface; `index.html` is the source.

```powershell
npm run check
npx --yes hyperframes@0.8.107 snapshot --at 2.7,6,8.8,12.5,16,19.5
```

Export only when requested:

```powershell
npx --yes hyperframes@0.8.107 render --output renders/winroll-promo.mp4
```

MP4 export requires FFmpeg and FFprobe on PATH. Preview and verification do not require an export.

Narration: local Kokoro `af_nova`, speed 0.68, 16.213375 seconds. Exact words are in `narration.txt`. Timing was measured locally with faster-whisper `base.en` and imported into HyperFrames; `transcript.json` holds word-level timestamps. Two short voice samples are kept locally in ignored audition files. Python dependencies are isolated in `.venv/`, and cached models/diagnostics are in ignored `.hyperframes/`.

To regenerate timing after replacing `narration.wav`, run `.venv/Scripts/python.exe timing.py`, then `npx --yes hyperframes@0.8.107 transcribe transcript-raw.json`. `timing.py` uses the bundled imageio FFmpeg to resample speech for the timing model. It does not export the video.

Local source assets were copied unchanged after the capture downloader failed to copy localhost media. Tabler Icons' MIT license is included under `capture/assets/LICENSE`. The gesture illustrations are the existing image-generated assets from the landing page.

The original page HTML is retained as `capture/extracted/page-source.txt` so Studio does not lint the captured website as a video composition.
