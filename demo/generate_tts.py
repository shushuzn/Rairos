"""
Generate compressed TTS narration to fit the 18-second video.
Each segment is read at ~3x speed so it fits the allocated time window.
"""

import asyncio, os, subprocess, glob

DEMO_DIR = r"D:\OpenClaw\workspace\80-PROJECTS\ai_research_os\demo"
AUDIO_DIR = DEMO_DIR
FFMPEG = r"C:\Users\adm\AppData\Local\Programs\Python\Python312\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v7.1.exe"

# Each segment must fit within its time window in the video
# Video: 18 seconds total, narration plays 0-18s
NARRATION_SCRIPT = [
    (0.0, 1.5, "Welcome to the AI Research OS Gap Feedback demonstration."),
    (1.5, 3.5, "This system automatically discovers research gaps and evolves knowledge."),
    (3.5, 5.5, "Topic Agent receives a query about transformer efficiency."),
    (5.5, 7.5, "Papers are retrieved and ranked by relevance."),
    (7.5, 9.5, "Gap Analyzer identifies missing connections."),
    (9.5, 11.5, "New directions become capsule genes."),
    (11.5, 13.5, "Genes stored in the evolving knowledge pool."),
    (13.5, 15.5, "Feedback loop continuously refines understanding."),
    (15.5, 17.5, "System converges on novel research opportunities."),
    (17.5, 18.0, "AI Research OS — advancing science, one gap at a time."),
]

VOICE = "en-US-AriaNeural"
RATE = "+100%"  # Double speed to fit time windows


async def generate_segment(text, output_path):
    import edge_tts
    communicate = edge_tts.Communicate(text, VOICE, rate=RATE)
    await communicate.save(output_path)
    return output_path


async def main():
    print("Generating fast TTS narration...")
    segs = []

    for i, (start, end, text) in enumerate(NARRATION_SCRIPT):
        out = os.path.join(AUDIO_DIR, f"tts_nar_{i:02d}.mp3")
        await generate_segment(text, out)
        segs.append(out)
        print(f"  [{i}] {text[:50]}...")
        await asyncio.sleep(0.1)

    # Combine using ffmpeg concat
    concat_list = os.path.join(AUDIO_DIR, "concat_list.txt")
    with open(concat_list, "w") as f:
        for seg in segs:
            f.write(f"file '{seg}'\n")

    out = os.path.join(AUDIO_DIR, "tts_narration.mp3")
    subprocess.run(
        [FFMPEG, "-f", "concat", "-safe", "0", "-i", concat_list,
         "-acodec", "libmp3lame", "-b:a", "128k", out],
        capture_output=True
    )
    os.remove(concat_list)

    # Check total duration
    result = subprocess.run([FFMPEG, "-i", out], capture_output=True, text=True)
    for line in result.stderr.split("\n"):
        if "Duration:" in line:
            print(f"\nTotal narration duration: {line.strip()}")
            break

    print(f"Output: {out}")


if __name__ == "__main__":
    asyncio.run(main())
