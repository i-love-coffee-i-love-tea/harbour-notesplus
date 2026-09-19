#!/usr/bin/env python3
"""Generate the harbour-notesplus app icon: ASCII fish ><(((º> on lined notepad background.

Run from the project root:
    python3 scripts/generate-icon.py

Requires: Pillow (pip install Pillow)
Output:   rpm/icons/{86x86,108x108,128x128,172x172}/harbour-notesplus.png
          rpm/harbour-notesplus.png (86x86 copy)
          notesplus-core/assets/web/icon.png
"""

import os
import sys

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    sys.exit("Pillow not found. Install with: pip install Pillow")

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RPM_DIR = os.path.join(PROJECT_ROOT, "rpm")
SCRIPTS_DIR = os.path.join(PROJECT_ROOT, "scripts")
CORE_DIR = os.path.join(PROJECT_ROOT, "notesplus-core")

SIZES = [86, 108, 128, 172]
CHARCOAL = (45, 52, 54, 255)  # High-contrast charcoal ink
FISH = "><(((\u00ba>"

# 512x512 master base image
MASTER_SIZE = 512
BASE_IMAGE_PATH = os.path.join(SCRIPTS_DIR, "icon-base.png")
if not os.path.exists(BASE_IMAGE_PATH):
    # Fallback to /tmp if run during generation
    BASE_IMAGE_PATH = "/tmp/test-uncommented.png"

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

# Load base notepad image
if os.path.exists(BASE_IMAGE_PATH):
    master = Image.open(BASE_IMAGE_PATH).convert("RGBA")
    if master.size != (MASTER_SIZE, MASTER_SIZE):
        master = master.resize((MASTER_SIZE, MASTER_SIZE), Image.LANCZOS)
else:
    sys.exit(f"Base icon image not found: {BASE_IMAGE_PATH}")

draw = ImageDraw.Draw(master)

# Overlay ASCII fish on the notepad paper area (below the top teal header bar at y ~ 85)
# Fish is rendered 20% taller without changing its width for increased readability and presence
base_font = find_font(95)
bbox_1x = ImageDraw.Draw(Image.new("RGBA", (1, 1))).textbbox((0, 0), FISH, font=base_font)
target_w = bbox_1x[2] - bbox_1x[0]
target_h = int(round((bbox_1x[3] - bbox_1x[1]) * 1.20))

# Supersample at 4x for clean vector-like glyph curves
SUPERSAMPLE = 4
font_hi = find_font(95 * SUPERSAMPLE)
bbox_hi = ImageDraw.Draw(Image.new("RGBA", (1, 1))).textbbox((0, 0), FISH, font=font_hi)
w_hi = bbox_hi[2] - bbox_hi[0]
h_hi = bbox_hi[3] - bbox_hi[1]

txt_canvas = Image.new("RGBA", (w_hi + 20, h_hi + 20), (0, 0, 0, 0))
d_hi = ImageDraw.Draw(txt_canvas)
d_hi.text((-bbox_hi[0], -bbox_hi[1]), FISH, fill=CHARCOAL, font=font_hi)

# Crop tight to glyph bounds and scale to target dimensions (width unchanged, height +20%)
txt_cropped = txt_canvas.crop(txt_canvas.getbbox())
fish_img = txt_cropped.resize((target_w, target_h), Image.LANCZOS)

# Position centered horizontally and vertically in paper pad area (between y=85 and y=512)
x = (MASTER_SIZE - target_w) // 2
y = 85 + (MASTER_SIZE - 85 - target_h) // 2
master.alpha_composite(fish_img, (x, y))

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

        # Update web assets icon
        web_icon_path = os.path.join(CORE_DIR, "assets", "web", "icon.png")
        if os.path.exists(os.path.dirname(web_icon_path)):
            icon.save(web_icon_path)
            print(f"  {web_icon_path}")

        # Update example note icons
        for ex in ["advanced-demo", "cheat-sheet"]:
            ex_icon_path = os.path.join(CORE_DIR, "examples", ex, "icon.png")
            if os.path.exists(os.path.dirname(ex_icon_path)):
                icon.save(ex_icon_path)
                print(f"  {ex_icon_path}")

print("Done — all icon sizes generated.")
