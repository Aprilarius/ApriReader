from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageOps

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "docs" / "steam" / "assets"
SOURCE = ASSETS / "source-key-art.png"
IVORY = (247, 243, 234, 255)
BRASS = (184, 138, 56, 255)
CHARCOAL = (27, 29, 28, 255)
WALNUT = (91, 59, 42, 255)
SOURCE_SIZE = (3072, 2048)

SIZES = {
    "store-header.png": (920, 430),
    "store-small.png": (462, 174),
    "store-main.png": (1232, 706),
    "store-vertical.png": (748, 896),
    "library-capsule.png": (600, 900),
    "library-header.png": (920, 430),
    "library-hero.png": (3840, 1240),
    "page-background.png": (1438, 810),
}


def font(size: int) -> ImageFont.FreeTypeFont:
    candidates = [
        Path("C:/Windows/Fonts/palabi.ttf"),
        Path("C:/Windows/Fonts/georgiab.ttf"),
        Path("C:/Windows/Fonts/timesbd.ttf"),
    ]
    path = next((item for item in candidates if item.is_file()), None)
    if path is None:
        raise RuntimeError("A compatible system serif font was not found.")
    return ImageFont.truetype(str(path), size)


def crop(source: Image.Image, size: tuple[int, int]) -> Image.Image:
    centering = (0.52, 0.58) if size[0] >= size[1] else (0.5, 0.56)
    return ImageOps.fit(
        source, size, method=Image.Resampling.LANCZOS, centering=centering
    )


def make_source_key_art() -> Image.Image:
    """Draw the original ApriReader key art from the project's own brand shapes."""
    width, height = SOURCE_SIZE
    image = Image.new("RGBA", SOURCE_SIZE, CHARCOAL)
    draw = ImageDraw.Draw(image)

    for y in range(height):
        ratio = y / (height - 1)
        red = round(27 + 35 * ratio)
        green = round(29 + 18 * ratio)
        blue = round(28 + 11 * ratio)
        draw.line((0, y, width, y), fill=(red, green, blue, 255))

    glow = Image.new("RGBA", SOURCE_SIZE)
    glow_draw = ImageDraw.Draw(glow)
    glow_draw.ellipse(
        (width * 0.17, height * 0.01, width * 0.83, height * 1.18),
        fill=(212, 173, 98, 78),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(170))
    image = Image.alpha_composite(image, glow)
    draw = ImageDraw.Draw(image)

    margin = 132
    for index in range(7):
        inset = margin + index * 42
        alpha = max(20, 82 - index * 10)
        draw.rounded_rectangle(
            (inset, inset, width - inset, height - inset),
            radius=96,
            outline=(184, 138, 56, alpha),
            width=3,
        )

    center_x = width // 2
    center_y = height // 2 + 40
    ring_radius = 690
    draw.ellipse(
        (
            center_x - ring_radius,
            center_y - ring_radius,
            center_x + ring_radius,
            center_y + ring_radius,
        ),
        outline=(212, 173, 98, 210),
        width=18,
    )

    shadow = Image.new("RGBA", SOURCE_SIZE)
    shadow_draw = ImageDraw.Draw(shadow)
    shadow_draw.ellipse(
        (center_x - 1000, center_y + 500, center_x + 1000, center_y + 790),
        fill=(0, 0, 0, 150),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(90))
    image = Image.alpha_composite(image, shadow)
    draw = ImageDraw.Draw(image)

    left_page = [
        (center_x, center_y - 430),
        (center_x - 280, center_y - 570),
        (center_x - 920, center_y - 500),
        (center_x - 1030, center_y + 500),
        (center_x - 440, center_y + 420),
        (center_x, center_y + 650),
    ]
    right_page = [(width - x, y) for x, y in left_page]
    draw.polygon(left_page, fill=(243, 239, 230, 245))
    draw.polygon(right_page, fill=(247, 243, 234, 250))
    draw.line(left_page + [left_page[0]], fill=(212, 173, 98, 255), width=14)
    draw.line(right_page + [right_page[0]], fill=(212, 173, 98, 255), width=14)

    for offset in range(0, 6):
        y = center_y - 300 + offset * 120
        fade = 112 - offset * 10
        draw.line(
            (center_x - 810, y, center_x - 175, y + 58),
            fill=(91, 59, 42, fade),
            width=8,
        )
        draw.line(
            (center_x + 175, y + 58, center_x + 810, y),
            fill=(91, 59, 42, fade),
            width=8,
        )

    draw.line(
        (center_x, center_y - 425, center_x, center_y + 650),
        fill=BRASS,
        width=34,
    )
    draw.ellipse(
        (center_x - 27, center_y - 457, center_x + 27, center_y - 403),
        fill=BRASS,
    )
    draw.ellipse(
        (center_x - 27, center_y + 620, center_x + 27, center_y + 674),
        fill=BRASS,
    )

    for x, y, radius in (
        (430, 420, 8),
        (2620, 370, 11),
        (370, 1660, 12),
        (2720, 1590, 8),
        (2450, 720, 6),
        (630, 790, 6),
    ):
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=BRASS)

    return image


def logo_layer(size: tuple[int, int], compact: bool = False) -> Image.Image:
    layer = Image.new("RGBA", size)
    draw = ImageDraw.Draw(layer)
    text_size = max(30, round(size[1] * (0.26 if compact else 0.18)))
    title = "ApriReader"
    while True:
        title_font = font(text_size)
        bounds = draw.textbbox((0, 0), title, font=title_font)
        text_width = bounds[2] - bounds[0]
        mark_size = round(text_size * 0.8)
        gap = round(text_size * 0.25)
        total_width = mark_size + gap + text_width
        if total_width <= size[0] * 0.82 or text_size <= 30:
            break
        text_size -= 2
    left = (size[0] - total_width) // 2
    top = round(size[1] * (0.1 if compact else 0.08))
    padding = round(text_size * 0.22)
    draw.rounded_rectangle(
        (
            left - padding,
            top - padding,
            left + total_width + padding,
            top + text_size + padding,
        ),
        radius=round(text_size * 0.16),
        fill=(27, 29, 28, 214),
        outline=(184, 138, 56, 150),
        width=max(1, round(text_size * 0.018)),
    )
    mark_box = (left, top, left + mark_size, top + mark_size)
    draw.ellipse(mark_box, outline=BRASS, width=max(2, round(text_size * 0.035)))
    inset = round(mark_size * 0.25)
    middle = left + mark_size // 2
    book_top = top + inset
    book_bottom = top + mark_size - inset
    draw.line(
        (left + inset, book_top, middle, book_top + 3, middle, book_bottom),
        fill=IVORY,
        width=max(2, round(text_size * 0.025)),
    )
    draw.line(
        (left + mark_size - inset, book_top, middle, book_top + 3),
        fill=IVORY,
        width=max(2, round(text_size * 0.025)),
    )
    draw.text(
        (left + mark_size + gap, top - round(text_size * 0.07)),
        title,
        font=title_font,
        fill=IVORY,
        stroke_width=max(1, round(text_size * 0.008)),
        stroke_fill=CHARCOAL,
    )
    return layer


def make_capsules(source: Image.Image) -> None:
    for name, size in SIZES.items():
        result = crop(source, size).convert("RGBA")
        if name not in {"library-hero.png", "page-background.png"}:
            result.alpha_composite(
                logo_layer(size, compact=name == "store-small.png")
            )
        result.convert("RGB").save(ASSETS / name, quality=96)


def make_transparent_logo() -> None:
    size = (1280, 360)
    layer = logo_layer(size)
    layer.save(ASSETS / "library-logo.png")


def validate() -> None:
    expected = {**SIZES, "library-logo.png": (1280, 360)}
    for name, size in expected.items():
        with Image.open(ASSETS / name) as image:
            if image.size != size:
                raise RuntimeError(f"{name}: expected {size}, got {image.size}")
    with Image.open(ASSETS / "library-logo.png") as logo:
        if logo.mode != "RGBA":
            raise RuntimeError("library-logo.png must preserve transparency")


def main() -> None:
    ASSETS.mkdir(parents=True, exist_ok=True)
    source = make_source_key_art()
    source.convert("RGB").save(SOURCE, quality=96)
    make_capsules(source.convert("RGB"))
    make_transparent_logo()
    validate()


if __name__ == "__main__":
    main()
