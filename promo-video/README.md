# SiteDatum promotional video

This directory contains the editable source and deterministic renderer for the 30-second SiteDatum promotional video.

## Output

- Resolution: 1920 x 1080
- Frame rate: 30 FPS
- Duration: 30 seconds
- Codec: H.264 High Profile, yuv420p, BT.709
- Audio: silent by design; no copyrighted third-party audio is included

The final master is written to `output/SiteDatum-Promo-30s.mp4`.

## Rebuild

From the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\promo-video\render.ps1
```

The local virtual environment contains Pillow and `imageio-ffmpeg`, which supplies the reproducible FFmpeg binary.

To recreate that environment if it is removed:

```powershell
python -m venv promo-video\.venv
.\promo-video\.venv\Scripts\python.exe -m pip install pillow imageio-ffmpeg
```

## Editable files

- `src/storyboard.json` controls scene timing, source images, captions, and camera movement.
- `src/render_video.py` contains the visual system, transitions, typography, framing, and encoder settings.
- `assets/` contains copies of the supplied SiteDatum screenshots and transparent logos. The original files are not modified.

## Animation approach

The renderer creates each 1080p frame directly with Pillow and streams raw RGB frames into FFmpeg. Application screenshots retain their native aspect ratio inside a softly rounded viewport. Motion is limited to 1.8-3.5% camera pushes or restrained horizontal pans. Scenes use one consistent 400 ms crossfade and a fixed caption rail, preserving readability and visual continuity.

The opening and closing cards use the supplied transparent SiteDatum logos over a branded navy field with restrained grid lines, cyan geometry, and a single orange activation pulse. No generated interface, stock footage, third-party music, or external brand assets are used.
