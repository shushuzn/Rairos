# Demo Tools

This directory contains tools for recording Rairos demos and showcase videos.

## demo_script.py — Browser Automation

Automates browser navigation through all major Rairos pages, useful for timing a demo before recording.

```bash
# Start the web UI first
python -m uvicorn web.app_new:app --port 8765

# In another terminal, run the automation
python tools/demo_script.py --url http://localhost:8765 --output demo_steps.json --visible
```

## demo_recorder.py — Screen Recording

Captures your screen and saves it as a GIF or MP4. Requires `mss` and `pillow`.

```bash
pip install mss pillow

# Record 30 seconds as GIF
python -m tools.demo_recorder --output demo.gif --duration 30 --fps 10

# Record 60 seconds as MP4 (requires ffmpeg)
python -m tools.demo_recorder --output demo.mp4 --duration 60 --fps 15
```

## Recommended Workflow

1. Start Rairos: `python -m uvicorn web.app_new:app --port 8765`
2. Run `demo_script.py --visible` to warm up all pages (caches embeddings, etc.)
3. Use OBS Studio or `demo_recorder.py` to capture the screen
4. Post-process with a tool like ezgif.com (GIF compression) or Handbrake (MP4)

## Manual Recording with OBS (recommended)

OBS Studio (obsproject.com) gives you the best quality:

1. Scene: Window capture → select browser window running Rairos
2. Set 1280×720 or 1920×1080, 30fps
3. Record the terminal + browser side-by-side
4. Use OBS filters: color correction, crop
5. Export as MP4 → upload to YouTube or convert to GIF with ezgif.com
