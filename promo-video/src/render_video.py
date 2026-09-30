from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont
import imageio_ffmpeg


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
OUTPUT = ROOT / "output"
STORYBOARD = Path(__file__).with_name("storyboard.json")

NAVY = (7, 24, 40)
NAVY_2 = (11, 39, 63)
TEAL = (27, 191, 199)
TEAL_SOFT = (105, 221, 224)
ORANGE = (239, 132, 52)
WHITE = (247, 250, 250)
MUTED = (157, 185, 198)
LINE = (34, 72, 93)


def clamp(value: float, low: float = 0.0, high: float = 1.0) -> float:
    return max(low, min(high, value))


def smooth(value: float) -> float:
    value = clamp(value)
    return value * value * (3.0 - 2.0 * value)


def ease_out(value: float) -> float:
    return 1.0 - (1.0 - clamp(value)) ** 3


def load_font(size: int, semibold: bool = False) -> ImageFont.FreeTypeFont:
    windir = Path(os.environ.get("WINDIR", "C:/Windows"))
    candidates = [
        windir / "Fonts" / ("seguisb.ttf" if semibold else "segoeui.ttf"),
        windir / "Fonts" / "arial.ttf",
    ]
    for candidate in candidates:
        if candidate.exists():
            return ImageFont.truetype(str(candidate), size=size)
    return ImageFont.load_default(size=size)


def alpha_scaled(image: Image.Image, opacity: float) -> Image.Image:
    result = image.copy().convert("RGBA")
    alpha = result.getchannel("A").point(lambda px: int(px * clamp(opacity)))
    result.putalpha(alpha)
    return result


def contain(image: Image.Image, width: int, height: int) -> Image.Image:
    ratio = min(width / image.width, height / image.height)
    size = (max(1, round(image.width * ratio)), max(1, round(image.height * ratio)))
    return image.resize(size, Image.Resampling.LANCZOS)


def wrapped_lines(draw: ImageDraw.ImageDraw, text: str, font: ImageFont.FreeTypeFont, max_width: int) -> list[str]:
    words = text.split()
    lines: list[str] = []
    current = ""
    for word in words:
        proposal = word if not current else f"{current} {word}"
        if draw.textlength(proposal, font=font) <= max_width:
            current = proposal
        else:
            if current:
                lines.append(current)
            current = word
    if current:
        lines.append(current)
    return lines


class Renderer:
    def __init__(self, storyboard: dict):
        self.storyboard = storyboard
        self.width, self.height = storyboard["size"]
        self.fps = storyboard["fps"]
        self.transition = storyboard["transition"]
        self.scenes = storyboard["scenes"]
        self.assets: dict[str, Image.Image] = {}
        for scene in self.scenes:
            asset = scene.get("asset")
            if asset:
                self.assets[asset] = Image.open(ASSETS / asset).convert("RGB")
        self.icon = Image.open(ASSETS / "Single logo.png").convert("RGBA")
        self.logo = Image.open(ASSETS / "Full logo.png").convert("RGBA")
        self.font_small = load_font(21, semibold=True)
        self.font_kicker = load_font(22, semibold=True)
        self.font_caption = load_font(48, semibold=True)
        self.font_meta = load_font(18)
        self.font_tagline = load_font(52, semibold=True)
        self.font_secondary = load_font(24)
        self.background = self._make_background()

    def _make_background(self) -> Image.Image:
        image = Image.new("RGB", (self.width, self.height), NAVY)
        pixels = image.load()
        for y in range(self.height):
            vertical = y / max(1, self.height - 1)
            for x in range(self.width):
                radial = max(0.0, 1.0 - math.dist((x, y), (420, 300)) / 1500.0)
                mix = clamp(0.08 + 0.26 * radial - 0.06 * vertical)
                pixels[x, y] = tuple(round(NAVY[i] * (1 - mix) + NAVY_2[i] * mix) for i in range(3))
        draw = ImageDraw.Draw(image)
        for x in range(0, self.width, 120):
            draw.line((x, 0, x, self.height), fill=(10, 37, 57), width=1)
        for y in range(0, self.height, 120):
            draw.line((0, y, self.width, y), fill=(10, 37, 57), width=1)
        draw.line((0, 78, self.width, 78), fill=LINE, width=1)
        return image

    def render(self, time_seconds: float) -> Image.Image:
        index = len(self.scenes) - 1
        for candidate, scene in enumerate(self.scenes):
            if scene["start"] <= time_seconds < scene["end"]:
                index = candidate
                break
        scene = self.scenes[index]
        local = time_seconds - scene["start"]
        current = self._render_scene(scene, local, index)
        if index > 0 and local < self.transition:
            previous = self.scenes[index - 1]
            previous_local = previous["end"] - previous["start"] - self.transition + local
            under = self._render_scene(previous, previous_local, index - 1)
            return Image.blend(under, current, smooth(local / self.transition))
        return current

    def _render_scene(self, scene: dict, local: float, index: int) -> Image.Image:
        if scene["kind"] == "opener":
            return self._render_opener(scene, local)
        if scene["kind"] == "final":
            return self._render_final(scene, local)
        return self._render_screen(scene, local, index)

    def _render_opener(self, scene: dict, local: float) -> Image.Image:
        frame = self.background.copy().convert("RGBA")
        draw = ImageDraw.Draw(frame)
        intro = ease_out(local / 1.0)
        icon_size = round(235 * (0.94 + 0.06 * intro))
        icon = contain(self.icon, icon_size, icon_size)
        icon_alpha = smooth(local / 0.7) * (1.0 - smooth((local - 1.05) / 0.45))
        frame.alpha_composite(alpha_scaled(icon, icon_alpha), ((self.width - icon.width) // 2, 330))

        logo_alpha = smooth((local - 1.0) / 0.7)
        logo = contain(self.logo, 900, 275)
        logo_y = 350 + round(10 * (1.0 - ease_out((local - 1.0) / 0.8)))
        logo_x = (self.width - logo.width) // 2
        frame.alpha_composite(alpha_scaled(logo, logo_alpha), (logo_x, logo_y))

        if 1.65 < local < 2.55:
            pulse = (local - 1.65) / 0.9
            radius = 18 + 24 * pulse
            alpha = int(130 * (1.0 - pulse))
            center_x = logo_x + round(165 * logo.width / 900)
            center_y = logo_y + logo.height // 2
            draw.ellipse((center_x - radius, center_y - radius, center_x + radius, center_y + radius), outline=(*ORANGE, alpha), width=3)

        text_alpha = smooth((local - 1.65) / 0.55)
        caption = scene["caption"]
        bbox = draw.textbbox((0, 0), caption, font=self.font_secondary)
        text_layer = Image.new("RGBA", frame.size, (0, 0, 0, 0))
        text_draw = ImageDraw.Draw(text_layer)
        text_draw.text(((self.width - (bbox[2] - bbox[0])) // 2, 674), caption, font=self.font_secondary, fill=(*MUTED, int(255 * text_alpha)))
        frame = Image.alpha_composite(frame, text_layer)
        self._draw_corner_marks(frame, text_alpha)
        return frame.convert("RGB")

    def _render_screen(self, scene: dict, local: float, index: int) -> Image.Image:
        duration = scene["end"] - scene["start"]
        progress = clamp(local / duration)
        frame = self.background.copy().convert("RGBA")
        viewport_x, viewport_y = 54, 60
        viewport_w, viewport_h = 1378, 957
        entry = ease_out(local / 0.5)
        horizontal_offset = round(24 * (1.0 - entry))
        if scene.get("motion") == "left":
            horizontal_offset *= -1
        viewport_x += horizontal_offset

        shadow = Image.new("RGBA", frame.size, (0, 0, 0, 0))
        shadow_draw = ImageDraw.Draw(shadow)
        shadow_draw.rounded_rectangle(
            (viewport_x + 10, viewport_y + 16, viewport_x + viewport_w + 10, viewport_y + viewport_h + 16),
            radius=18,
            fill=(0, 0, 0, 145),
        )
        shadow = shadow.filter(ImageFilter.GaussianBlur(18))
        frame = Image.alpha_composite(frame, shadow)

        source = self.assets[scene["asset"]]
        zoom = 1.0 + 0.035 * smooth(progress)
        if scene.get("motion") in {"left", "right"}:
            zoom = 1.018
        target_w = round(viewport_w * zoom)
        target_h = round(target_w / (source.width / source.height))
        if target_h < viewport_h:
            target_h = round(viewport_h * zoom)
            target_w = round(target_h * (source.width / source.height))
        scaled = source.resize((target_w, target_h), Image.Resampling.LANCZOS)
        extra_x = max(0, target_w - viewport_w)
        extra_y = max(0, target_h - viewport_h)
        motion = scene.get("motion")
        if motion == "right":
            crop_x = round(extra_x * (0.25 + 0.5 * smooth(progress)))
        elif motion == "left":
            crop_x = round(extra_x * (0.75 - 0.5 * smooth(progress)))
        else:
            crop_x = extra_x // 2
        crop_y = round(extra_y * (0.42 + 0.10 * smooth(progress)))
        screen = scaled.crop((crop_x, crop_y, crop_x + viewport_w, crop_y + viewport_h)).convert("RGBA")
        mask = Image.new("L", (viewport_w, viewport_h), 0)
        ImageDraw.Draw(mask).rounded_rectangle((0, 0, viewport_w - 1, viewport_h - 1), radius=13, fill=255)
        screen.putalpha(mask)
        frame.alpha_composite(screen, (viewport_x, viewport_y))

        outline = ImageDraw.Draw(frame)
        outline.rounded_rectangle(
            (viewport_x - 1, viewport_y - 1, viewport_x + viewport_w, viewport_y + viewport_h),
            radius=14,
            outline=(50, 106, 125, 220),
            width=2,
        )
        self._draw_caption_rail(frame, scene, index, entry)
        return frame.convert("RGB")

    def _draw_caption_rail(self, frame: Image.Image, scene: dict, index: int, opacity: float) -> None:
        layer = Image.new("RGBA", frame.size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(layer)
        x = 1500
        alpha = int(255 * opacity)
        draw.line((x, 283, x + 58, 283), fill=(*ORANGE, alpha), width=4)
        draw.text((x, 315), scene["kicker"], font=self.font_kicker, fill=(*TEAL_SOFT, alpha))
        y = 365
        for line in wrapped_lines(draw, scene["caption"], self.font_caption, 340):
            draw.text((x, y), line, font=self.font_caption, fill=(*WHITE, alpha))
            y += 61
        draw.text((x, 874), "THE PROJECT RECORD YOU CONTROL", font=self.font_meta, fill=(*MUTED, alpha))
        draw.text((x, 946), f"{index:02d}", font=self.font_small, fill=(*ORANGE, alpha))
        draw.line((x + 45, 960, 1845, 960), fill=(*LINE, alpha), width=2)
        progress_right = x + 45 + round(300 * index / 8)
        draw.line((x + 45, 960, progress_right, 960), fill=(*TEAL, alpha), width=3)
        frame.alpha_composite(layer)

    def _render_final(self, scene: dict, local: float) -> Image.Image:
        frame = self.background.copy().convert("RGBA")
        draw = ImageDraw.Draw(frame)
        fade = smooth(local / 0.55)
        logo = contain(self.logo, 850, 258)
        logo_x = (self.width - logo.width) // 2
        logo_y = 310 + round(12 * (1.0 - ease_out(local / 0.8)))
        frame.alpha_composite(alpha_scaled(logo, fade), (logo_x, logo_y))

        tagline_alpha = smooth((local - 0.45) / 0.55)
        tagline = scene["caption"]
        bbox = draw.textbbox((0, 0), tagline, font=self.font_tagline)
        tag_x = (self.width - (bbox[2] - bbox[0])) // 2
        draw.text((tag_x, 608), tagline, font=self.font_tagline, fill=(*WHITE, int(255 * tagline_alpha)))
        secondary = scene["secondary"]
        secondary_bbox = draw.textbbox((0, 0), secondary, font=self.font_secondary)
        secondary_x = (self.width - (secondary_bbox[2] - secondary_bbox[0])) // 2
        draw.text((secondary_x, 686), secondary, font=self.font_secondary, fill=(*MUTED, int(255 * tagline_alpha)))
        draw.line((830, 765, 1090, 765), fill=(*TEAL, int(220 * tagline_alpha)), width=2)

        if 1.25 < local < 2.15:
            pulse = (local - 1.25) / 0.9
            radius = 16 + 22 * pulse
            alpha = int(115 * (1.0 - pulse))
            center_x = logo_x + round(155 * logo.width / 850)
            center_y = logo_y + logo.height // 2
            draw.ellipse((center_x - radius, center_y - radius, center_x + radius, center_y + radius), outline=(*ORANGE, alpha), width=3)
        self._draw_corner_marks(frame, fade)
        return frame.convert("RGB")

    def _draw_corner_marks(self, frame: Image.Image, opacity: float) -> None:
        draw = ImageDraw.Draw(frame)
        alpha = int(150 * clamp(opacity))
        draw.line((72, 72, 150, 72), fill=(*TEAL, alpha), width=2)
        draw.line((72, 72, 72, 150), fill=(*TEAL, alpha), width=2)
        draw.line((self.width - 72, self.height - 72, self.width - 150, self.height - 72), fill=(*ORANGE, alpha), width=2)
        draw.line((self.width - 72, self.height - 72, self.width - 72, self.height - 150), fill=(*ORANGE, alpha), width=2)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render the SiteDatum 30-second promotional video.")
    parser.add_argument("--output", type=Path, default=OUTPUT / "SiteDatum-Promo-30s.mp4")
    parser.add_argument("--crf", type=int, default=18, help="H.264 quality: lower is higher quality.")
    parser.add_argument("--preset", default="slow", help="FFmpeg x264 preset.")
    parser.add_argument("--preview-frame", type=float, help="Render one PNG frame at the specified second instead of the MP4.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    storyboard = json.loads(STORYBOARD.read_text(encoding="utf-8"))
    renderer = Renderer(storyboard)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    if args.preview_frame is not None:
        preview = renderer.render(args.preview_frame)
        preview_path = OUTPUT / f"preview-{args.preview_frame:05.2f}s.png"
        preview.save(preview_path)
        print(preview_path)
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    ffmpeg = imageio_ffmpeg.get_ffmpeg_exe()
    command = [
        ffmpeg,
        "-y",
        "-f", "rawvideo",
        "-vcodec", "rawvideo",
        "-pix_fmt", "rgb24",
        "-s", f"{renderer.width}x{renderer.height}",
        "-r", str(renderer.fps),
        "-i", "-",
        "-an",
        "-c:v", "libx264",
        "-preset", args.preset,
        "-crf", str(args.crf),
        "-pix_fmt", "yuv420p",
        "-profile:v", "high",
        "-level", "4.1",
        "-color_primaries", "bt709",
        "-color_trc", "bt709",
        "-colorspace", "bt709",
        "-movflags", "+faststart",
        str(args.output),
    ]
    process = subprocess.Popen(command, stdin=subprocess.PIPE)
    total_frames = round(storyboard["duration"] * renderer.fps)
    try:
        assert process.stdin is not None
        for frame_number in range(total_frames):
            time_seconds = frame_number / renderer.fps
            frame = renderer.render(time_seconds)
            process.stdin.write(frame.tobytes())
            if frame_number % renderer.fps == 0:
                print(f"Rendering {time_seconds:04.1f}s / {storyboard['duration']:.1f}s", flush=True)
        process.stdin.close()
        return_code = process.wait()
    except BaseException:
        process.kill()
        raise
    if return_code != 0:
        raise RuntimeError(f"FFmpeg exited with code {return_code}")
    print(args.output)
    return 0


if __name__ == "__main__":
    sys.exit(main())
