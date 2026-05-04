"""Demo recorder — capture Rairos web UI + CLI as video/GIF.

Usage:
    python -m tools.demo_recorder --output demo.gif --duration 30
    python -m tools.demo_recorder --output demo.mp4 --duration 60

Requires: mss (pip install mss pillow)
"""

import argparse
import time
from pathlib import Path

try:
    import mss
    import mss.tools
except ImportError:
    print("Error: mss not installed. Run: pip install mss pillow")
    raise SystemExit(1) from None


def record_screen(output: str, duration: int, fps: int = 10):
    """Record screen to video/GIF using mss."""
    output_path = Path(output)
    with mss.mss() as sct:
        monitor = sct.monitors[1]  # primary monitor
        interval = 1.0 / fps

        frames = []
        start = time.time()
        frame_num = 0

        print(f"Recording {duration}s from monitor {monitor} at {fps}fps...")
        print(f"Output: {output_path}")
        print("Press Ctrl+C to stop early.")

        try:
            while time.time() - start < duration:
                shot = sct.grab(monitor)
                frames.append(shot)
                frame_num += 1
                time.sleep(interval)
        except KeyboardInterrupt:
            print(f"\nStopped at {len(frames)} frames.")

    if not frames:
        print("No frames captured.")
        return

    ext = output_path.suffix.lower()
    if ext == ".gif":
        _write_gif(frames, output_path, fps)
    else:
        _write_mp4(frames, output_path, fps)

    print(f"Saved {len(frames)} frames → {output_path}")


def _write_gif(frames, output_path: Path, fps: int):
    """Convert frames to GIF using Pillow."""
    from PIL import Image

    images = []
    for frame in frames:
        img = Image.frombytes("RGB", frame.size, frame.bgra, "raw", "BGRX")
        images.append(img)

    images[0].save(
        output_path,
        save_all=True,
        append_images=images[1:],
        duration=int(1000 / fps),
        loop=0,
    )


def _write_mp4(frames, output_path: Path, fps: int):
    """Save frames as MP4 using ffmpeg via subprocess."""
    import subprocess
    import tempfile
    import os

    tmp_dir = Path(tempfile.mkdtemp())
    tmp_pattern = tmp_dir / "frame_%04d.png"

    for i, frame in enumerate(frames):
        img_path = tmp_dir / f"frame_{i:04d}.png"
        mss.tools.to_png(frame.rgb, frame.size, str(img_path))

    try:
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-framerate",
                str(fps),
                "-i",
                str(tmp_pattern),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-crf",
                "23",
                str(output_path),
            ],
            check=True,
            capture_output=True,
        )
    except FileNotFoundError:
        print("Error: ffmpeg not found. Install ffmpeg or use --gif format.")
        print("Frames saved to:", tmp_dir)
    except subprocess.CalledProcessError as e:
        print("ffmpeg error:", e.stderr.decode() if e.stderr else e)
    finally:
        for f in tmp_dir.iterdir():
            f.unlink()
        tmp_dir.rmdir()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Record Rairos demo")
    parser.add_argument("--output", "-o", default="demo.gif", help="Output file (.gif or .mp4)")
    parser.add_argument("--duration", "-d", type=int, default=30, help="Duration in seconds")
    parser.add_argument("--fps", type=int, default=10, help="Frames per second")
    args = parser.parse_args()

    record_screen(args.output, args.duration, args.fps)
