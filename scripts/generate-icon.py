#!/usr/bin/env python3
"""Generate the harbour-notesplusplus app icon: Notes++ on teal background.

Run from the project root:
    python3 scripts/generate-icon.py

Requires: Pillow (pip install Pillow)
Output:   rpm/harbour-notesplusplus.png (86x86)
"""

import os
import sys

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    sys.exit("Pillow not found. Install with: pip install Pillow")

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_PATH = os.path.join(PROJECT_ROOT, "rpm", "harbour-notesplusplus.png")

SIZE = 86
img = Image.new("RGB", (SIZE, SIZE), (0, 150, 136))
draw = ImageDraw.Draw(img)

FISH = "><(((\u00ba>"

# Try to find a monospace font, fall back to default
font = None
for path in [
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf",
    "/usr/share/fonts/dejavu/DejaVuSansMono-Bold.ttf",
]:
    if os.path.exists(path):
        font = ImageFont.truetype(path, 18)
        break

if font is None:
    font = ImageFont.load_default()

# Center the text with padding
bbox = draw.textbbox((0, 0), FISH, font=font)
tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
x = (SIZE - tw) // 2
y = (SIZE - th) // 2 - bbox[1]

draw.text((x, y), FISH, fill=(255, 255, 255), font=font)

img.save(OUT_PATH)
print(f"Icon written to {OUT_PATH}")
