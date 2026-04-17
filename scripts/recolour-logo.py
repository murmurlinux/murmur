#!/usr/bin/env python3
"""Recolour the Murmur logo from teal-on-dark to rust-on-cream.

Maps:
  Teal (#0fa697 family) -> Rust (#c9482b)
  Dark blue (#0f2b36 family) -> Cream (#f5f0e6)

Preserves luminance variation and alpha channel.
Generates all required icon sizes.
"""

import colorsys
import os
from PIL import Image
import struct

# Source and target colours
# Old palette (by hue range)
TEAL_HUE_RANGE = (150, 200)  # degrees - the "m" and waves
DARK_HUE_RANGE = (180, 210)  # degrees - background (also teal-ish but very dark)
DARK_LUMA_MAX = 0.15  # background pixels are very dark

# New palette
RUST = (0xc9, 0x48, 0x2b)   # accent colour
CREAM = (0xf5, 0xf0, 0xe6)  # background colour


def rgb_to_hsl(r, g, b):
    """Convert 0-255 RGB to 0-360/0-1/0-1 HSL."""
    h, l, s = colorsys.rgb_to_hls(r / 255, g / 255, b / 255)
    return h * 360, s, l


def lerp_colour(c1, c2, t):
    """Linearly interpolate between two RGB tuples."""
    return tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))


def recolour_pixel(r, g, b, a):
    """Map a single pixel from old palette to new palette."""
    if a == 0:
        return (r, g, b, a)

    h, s, l = rgb_to_hsl(r, g, b)

    # Classify: is this a background pixel or a foreground (teal) pixel?
    # Background pixels are very dark (low luminance)
    if l < DARK_LUMA_MAX:
        # Background - map to cream, preserving relative luminance
        # Scale luminance: old dark range (0-0.15) -> cream range
        # Cream is quite bright (l ~0.93), so map dark variations subtly
        base = CREAM
        # Slight darkening for darker background pixels
        factor = 1.0 - (DARK_LUMA_MAX - l) * 0.15
        new_rgb = tuple(max(0, min(255, int(c * factor))) for c in base)
        return (*new_rgb, a)
    else:
        # Foreground (teal "m", waves, border) - map to rust
        # Preserve luminance variation: brighter teal -> brighter rust
        base = RUST
        # The teal pixels vary in luminance; scale rust accordingly
        teal_base_l = 0.33  # approximate luminance of #0fa697
        rust_base_l = 0.30  # approximate luminance of #c9482b
        if teal_base_l > 0:
            lum_ratio = l / teal_base_l
        else:
            lum_ratio = 1.0
        # Clamp the ratio so we don't blow out
        lum_ratio = max(0.7, min(1.4, lum_ratio))
        new_rgb = tuple(max(0, min(255, int(c * lum_ratio))) for c in base)
        return (*new_rgb, a)


def recolour_image(img):
    """Recolour an entire RGBA image."""
    pixels = list(img.getdata())
    new_pixels = [recolour_pixel(*p) for p in pixels]
    new_img = Image.new("RGBA", img.size)
    new_img.putdata(new_pixels)
    return new_img


def create_ico(img, path):
    """Create a .ico file with multiple sizes."""
    sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    icons = [img.resize(s, Image.LANCZOS) for s in sizes]
    icons[0].save(path, format="ICO", sizes=sizes, append_images=icons[1:])


def main():
    source_path = os.path.expanduser(
        "~/Projects/murmur/design/logo/murmur-logo-source.png"
    )
    base_dir = os.path.expanduser("~/Projects/murmur")

    print(f"Loading source: {source_path}")
    source = Image.open(source_path).convert("RGBA")
    print(f"Source size: {source.size}")

    print("Recolouring...")
    recoloured = recolour_image(source)

    # Save source-resolution recoloured version
    recoloured.save(os.path.join(base_dir, "design/logo/murmur-logo-source.png"))
    print("Saved recoloured source")

    # Generate all sizes
    sizes = [16, 32, 48, 64, 128, 256, 512]
    for size in sizes:
        resized = recoloured.resize((size, size), Image.LANCZOS)

        # design/logo/
        path = os.path.join(base_dir, f"design/logo/murmur-logo-{size}.png")
        resized.save(path)
        print(f"Saved {path}")

        # src-tauri/icons/
        path = os.path.join(base_dir, f"src-tauri/icons/{size}x{size}.png")
        resized.save(path)
        print(f"Saved {path}")

    # icon.png (512x512)
    recoloured.resize((512, 512), Image.LANCZOS).save(
        os.path.join(base_dir, "src-tauri/icons/icon.png")
    )
    print("Saved src-tauri/icons/icon.png")

    # src/assets/logo.png (128x128 for recording popup)
    recoloured.resize((128, 128), Image.LANCZOS).save(
        os.path.join(base_dir, "src/assets/logo.png")
    )
    print("Saved src/assets/logo.png")

    # GitHub avatar (512x512)
    recoloured.resize((512, 512), Image.LANCZOS).save(
        os.path.join(base_dir, "design/logo/github-avatar.png")
    )
    print("Saved design/logo/github-avatar.png")

    # Favicon .ico
    create_ico(recoloured, os.path.join(base_dir, "design/logo/favicon.ico"))
    print("Saved design/logo/favicon.ico")

    print("\nDone! Review the recoloured images before committing.")


if __name__ == "__main__":
    main()
