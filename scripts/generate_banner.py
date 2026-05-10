"""Generate banner.png and favicon.png from SVG."""

import subprocess
import os

# Check if cairosvg is available
try:
    import cairosvg

    HAS_CAIRO = True
except ImportError:
    HAS_CAIRO = False

# SVG files
ASSETS = "/d/OpenClaw/workspace/80-PROJECTS/ai_research_os/docs/assets"
LOGO_SVG = f"{ASSETS}/logo.svg"
FAVICON_SVG = f"{ASSETS}/favicon.svg"

# Output files
LOGO_PNG = f"{ASSETS}/logo.png"
FAVICON_PNG = f"{ASSETS}/favicon.png"


def svg_to_png(svg_path, png_path, width=None, height=None):
    if not os.path.exists(svg_path):
        print(f"SVG not found: {svg_path}")
        return False
    if HAS_CAIRO:
        if width and height:
            cairosvg.svg2png(
                url=svg_path, write_to=png_path, output_width=width, output_height=height
            )
        elif width:
            cairosvg.svg2png(url=svg_path, write_to=png_path, output_width=width)
        elif height:
            cairosvg.svg2png(url=svg_path, write_to=png_path, output_height=height)
        else:
            cairosvg.svg2png(url=svg_path, write_to=png_path)
        print(f"Generated {png_path} with cairosvg")
        return True
    else:
        # Try inkscape
        if os.path.exists("C:/Program Files/Inkscape/bin/inkscape.exe"):
            exe = "C:/Program Files/Inkscape/bin/inkscape.exe"
            if width:
                subprocess.run(
                    [exe, svg_path, "-w", str(width), "-o", png_path],
                    check=True,
                    capture_output=True,
                )
            else:
                subprocess.run([exe, svg_path, "-o", png_path], check=True, capture_output=True)
            print(f"Generated {png_path} with Inkscape")
            return True
        elif os.path.exists("C:/Program Files (x86)/Inkscape/bin/inkscape.exe"):
            exe = "C:/Program Files (x86)/Inkscape/bin/inkscape.exe"
            subprocess.run(
                [exe, svg_path, "-w", str(width), "-o", png_path], check=True, capture_output=True
            )
            print(f"Generated {png_path} with Inkscape")
            return True
        print("No SVG converter found. Install cairosvg: pip install cairosvg")
        return False


# Generate logo.png (256x256)
svg_to_png(LOGO_SVG, LOGO_PNG, width=256)

# Generate favicon.png (32x32)
svg_to_png(FAVICON_SVG, FAVICON_PNG, width=32)
