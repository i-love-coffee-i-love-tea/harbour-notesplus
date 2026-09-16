#!/usr/bin/env python3
"""Generate the harbour-notesplus app icon: ><(((º> on teal background.

Run from the project root:
    python3 scripts/generate-icon.py

Requires: Pillow (pip install Pillow)
Output:   rpm/icons/{86x86,108x108,128x128,172x172}/harbour-notesplus.png
          rpm/harbour-notesplus.png (86x86 copy)
"""

import os
import sys

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    sys.exit("Pillow not found. Install with: pip install Pillow")

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RPM_DIR = os.path.join(PROJECT_ROOT, "rpm")

SIZES = [86, 108, 128, 172]
BACKGROUND = (0, 150, 136)  # Material Design Teal 600
TEXT_COLOR = (255, 255, 255)
FISH = "><(((\u00ba>"

# Generate at 512x512 master, then downscale
MASTER_SIZE = 512

# Find a monospace font
def find_font(size):
    for path in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf",
        "/usr/share/fonts/dejavu/DejaVuSansMono-Bold.ttf",
    ]:
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()

# Render at master resolution
master = Image.new("RGB", (MASTER_SIZE, MASTER_SIZE), BACKGROUND)
draw = ImageDraw.Draw(master)

font = find_font(110)
bbox = draw.textbbox((0, 0), FISH, font=font)
tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
x = (MASTER_SIZE - tw) // 2
y = (MASTER_SIZE - th) // 2 - bbox[1]
draw.text((x, y), FISH, fill=TEXT_COLOR, font=font)

# Generate all sizes
for size in SIZES:
    icon = master.resize((size, size), Image.LANCZOS)

    # Save to rpm/icons/<size>x<size>/
    icon_dir = os.path.join(RPM_DIR, "icons", f"{size}x{size}")
    os.makedirs(icon_dir, exist_ok=True)
    icon_path = os.path.join(icon_dir, "harbour-notesplus.png")
    icon.save(icon_path)
    print(f"  {icon_path}")

    # Also save 86x86 as the default rpm/harbour-notesplus.png
    if size == 86:
        default_path = os.path.join(RPM_DIR, "harbour-notesplus.png")
        icon.save(default_path)
        print(f"  {default_path} (default)")

print("Done — all icon sizes generated.")
