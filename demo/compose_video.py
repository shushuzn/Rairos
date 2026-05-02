"""
Compose final demo video with:
  - Intro + main gap-loop + outro (concatenated)
  - TTS audio (intro + narration + outro) muxed in
  - Hard-coded subtitles burned into video
"""

import subprocess, os

DEMO_DIR = r"D:\OpenClaw\workspace\80-PROJECTS\ai_research_os\demo"
FFMPEG = r"C:\Users\adm\AppData\Local\Programs\Python\Python312\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe"


def run(cmd, desc="", cwd=None):
    print(f"\n{'='*60}\n{desc}")
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    if result.returncode != 0:
        print(f"STDERR: {result.stderr[-1000:]}")
        raise SystemExit(f"FAILED: {desc}")
    else:
        print(f"  OK (exit 0)")
    return result


def get_duration(file):
    result = subprocess.run([FFMPEG, "-i", file], capture_output=True, text=True)
    for line in result.stderr.split("\n"):
        if "Duration:" in line:
            dur_str = line.split("Duration:")[1].split(",")[0].strip()
            h, m, s = dur_str.split(":")
            return float(h) * 3600 + float(m) * 60 + float(s)
    return None


# Paths
MAIN_VIDEO = os.path.join(DEMO_DIR, "gap-feedback-loop.mp4")
INTRO_VIDEO = os.path.join(DEMO_DIR, "intro.mp4")
OUTRO_VIDEO = os.path.join(DEMO_DIR, "outro.mp4")
TTS_INTRO = os.path.join(DEMO_DIR, "tts_intro.mp3")
TTS_NARR = os.path.join(DEMO_DIR, "tts_narration_18s.mp3")
TTS_OUTRO = os.path.join(DEMO_DIR, "tts_outro.mp3")
ASS_FILE = "gap-feedback-loop.ass"  # relative path for subtitles filter

FINAL_RAW = os.path.join(DEMO_DIR, "final_raw.mp4")
FINAL_WITH_AUDIO = os.path.join(DEMO_DIR, "final_with_audio.mp4")
FINAL_OUT = os.path.join(DEMO_DIR, "ai_research_os_demo.mp4")

print("Checking input files...")
for f in [MAIN_VIDEO, TTS_INTRO, TTS_NARR, TTS_OUTRO]:
    print(f"  {'ok' if os.path.exists(f) else 'MISSING'} {os.path.basename(f)}")

# Step 1: Generate intro video
intro_dur = get_duration(TTS_INTRO)
outro_dur = get_duration(TTS_OUTRO)
print(f"\nIntro: {intro_dur:.2f}s  Outro: {outro_dur:.2f}s")

run([
    FFMPEG, "-y",
    "-f", "lavfi", "-i", f"color=c=0x1a1a2e:s=1920x1080:d={intro_dur}:r=30",
    "-f", "lavfi", "-i", f"anullsrc=r=44100:cl=stereo:d={intro_dur}",
    "-vf", f"drawtext=text='AI Research OS':fontcolor=white:fontsize=80:borderw=5:bordercolor=black:x=(w-text_w)/2:y=(h-text_h)/2,fade=t=in:st=0:d=0.5",
    "-c:v", "libx264", "-preset", "fast", "-crf", "18",
    "-c:a", "aac", "-b:a", "128k", "-shortest",
    INTRO_VIDEO
], "Generating intro video")

# Step 2: Generate outro video
run([
    FFMPEG, "-y",
    "-f", "lavfi", "-i", f"color=c=0x0d0d1a:s=1920x1080:d={outro_dur}:r=30",
    "-f", "lavfi", "-i", f"anullsrc=r=44100:cl=stereo:d={outro_dur}",
    "-vf", f"drawtext=text='Thanks for Watching':fontcolor=white:fontsize=72:borderw=5:x=(w-text_w)/2:y=h/2-60,drawtext=text='Subscribe for more':fontcolor=0xaaaaaa:fontsize=48:borderw=3:x=(w-text_w)/2:y=h/2+10,fade=t=out:st={outro_dur-0.5}:d=0.5",
    "-c:v", "libx264", "-preset", "fast", "-crf", "18",
    "-c:a", "aac", "-b:a", "128k", "-shortest",
    OUTRO_VIDEO
], "Generating outro video")

# Step 3: Concatenate videos (copy streams - no re-encode)
concat_list = os.path.join(DEMO_DIR, "video_concat.txt")
with open(concat_list, "w") as f:
    f.write(f"file '{MAIN_VIDEO}'\n")  # no intro/outro to keep file size small
    # Note: intro + outro skipped to keep demo focused on core animation

# Actually, let's include intro+outro
with open(concat_list, "w") as f:
    f.write(f"file '{INTRO_VIDEO}'\n")
    f.write(f"file '{MAIN_VIDEO}'\n")
    f.write(f"file '{OUTRO_VIDEO}'\n")

run([
    FFMPEG, "-y", "-f", "concat", "-safe", "0",
    "-i", concat_list, "-c", "copy", FINAL_RAW
], "Concatenating videos")

# Step 4: Concatenate audio
audio_concat = os.path.join(DEMO_DIR, "audio_concat.txt")
with open(audio_concat, "w") as f:
    for af in [TTS_INTRO, TTS_NARR, TTS_OUTRO]:
        f.write(f"file '{af}'\n")

audio_out = os.path.join(DEMO_DIR, "full_audio.mp3")
run([
    FFMPEG, "-y", "-f", "concat", "-safe", "0",
    "-i", audio_concat,
    "-acodec", "libmp3lame", "-b:a", "192k", audio_out
], "Concatenating audio tracks")

# Step 5: Mux video + audio
run([
    FFMPEG, "-y",
    "-i", FINAL_RAW, "-i", audio_out,
    "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest",
    FINAL_WITH_AUDIO
], "Muxing video + audio")

# Step 6: Burn subtitles (run from DEMO_DIR so relative ASS path works)
run([
    FFMPEG, "-y",
    "-i", FINAL_WITH_AUDIO,
    "-vf", f"subtitles={ASS_FILE}",
    "-c:a", "copy",
    "-crf", "18", "-preset", "slow",
    FINAL_OUT
], "Burning subtitles", cwd=DEMO_DIR)

# Cleanup
for f in [concat_list, audio_concat, FINAL_RAW, FINAL_WITH_AUDIO, INTRO_VIDEO, OUTRO_VIDEO]:
    if os.path.exists(f):
        os.remove(f)

print(f"\n{'='*60}")
print(f"OUTPUT: {FINAL_OUT}")
sz = os.path.getsize(FINAL_OUT)
print(f"SIZE: {sz/1024/1024:.1f} MB")
dur = get_duration(FINAL_OUT)
print(f"DURATION: {dur:.1f}s" if dur else "")
print("DONE!")
