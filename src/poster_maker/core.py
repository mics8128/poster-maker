from __future__ import annotations

import argparse
import os
import tempfile
from dataclasses import dataclass
from pathlib import Path

import fitz  # PyMuPDF
from PIL import Image

A4_WIDTH_PT = 595.2755905512
A4_HEIGHT_PT = 841.8897637795
MM_TO_PT = 72 / 25.4
SUPPORTED_IMAGES = {".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tif", ".tiff"}


@dataclass(frozen=True)
class PosterOptions:
    cols: int = 2
    rows: int = 2
    overlap_mm: float = 10.0
    margin_mm: float = 8.0
    dpi: int = 200
    landscape: bool = False
    draw_cut_marks: bool = True
    draw_alignment_guides: bool = True
    draw_labels: bool = False
    page_index: int = 0
    auto_landscape: bool = False


@dataclass(frozen=True)
class GeneratedPage:
    col: int
    row: int
    clip: fitz.Rect


def page_size(landscape: bool) -> tuple[float, float]:
    return (A4_HEIGHT_PT, A4_WIDTH_PT) if landscape else (A4_WIDTH_PT, A4_HEIGHT_PT)


def choose_landscape(src_rect: fitz.Rect, cols: int, rows: int, margin_pt: float) -> bool:
    """Pick A4 portrait/landscape orientation with larger used poster area."""
    best_landscape = False
    best_score = -1.0
    for landscape in (False, True):
        w, h = page_size(landscape)
        printable = fitz.Rect(margin_pt, margin_pt, w - margin_pt, h - margin_pt)
        total_w = printable.width * cols
        total_h = printable.height * rows
        scale = min(total_w / src_rect.width, total_h / src_rect.height)
        score = (src_rect.width * scale) * (src_rect.height * scale)
        if score > best_score:
            best_score = score
            best_landscape = landscape
    return best_landscape


def mm(value: float) -> float:
    return value * MM_TO_PT


def load_source_as_pdf(path: str | Path, page_index: int = 0, dpi: int = 200) -> tuple[fitz.Document, int]:
    """Return source as PDF document and page number inside returned doc."""
    path = Path(path)
    suffix = path.suffix.lower()
    if suffix == ".pdf":
        doc = fitz.open(path)
        if doc.page_count == 0:
            raise ValueError("PDF has no pages")
        if not (0 <= page_index < doc.page_count):
            raise ValueError(f"Page index out of range: {page_index + 1} / {doc.page_count}")
        return doc, page_index

    if suffix not in SUPPORTED_IMAGES:
        raise ValueError(f"Unsupported input type: {suffix}")

    # Normalize through Pillow, preserves broad image support and avoids EXIF rotation surprises.
    img = Image.open(path)
    img = img.convert("RGB")
    w_px, h_px = img.size
    w_pt = w_px / dpi * 72
    h_pt = h_px / dpi * 72

    tmp = tempfile.NamedTemporaryFile(suffix=".png", delete=False)
    tmp_path = tmp.name
    tmp.close()
    img.save(tmp_path)

    doc = fitz.open()
    page = doc.new_page(width=w_pt, height=h_pt)
    page.insert_image(fitz.Rect(0, 0, w_pt, h_pt), filename=tmp_path, keep_proportion=False)
    try:
        os.unlink(tmp_path)
    except OSError:
        pass
    return doc, 0


def _source_fit_rect(src_rect: fitz.Rect, total_w: float, total_h: float) -> fitz.Rect:
    scale = min(total_w / src_rect.width, total_h / src_rect.height)
    fitted_w = src_rect.width * scale
    fitted_h = src_rect.height * scale
    x0 = (total_w - fitted_w) / 2
    y0 = (total_h - fitted_h) / 2
    return fitz.Rect(x0, y0, x0 + fitted_w, y0 + fitted_h)


def _tile_clip(fitted_rect: fitz.Rect, col: int, row: int, cols: int, rows: int, overlap: float) -> fitz.Rect:
    tile_w = fitted_rect.width / cols
    tile_h = fitted_rect.height / rows
    x0 = fitted_rect.x0 + col * tile_w - (overlap if col > 0 else 0)
    y0 = fitted_rect.y0 + row * tile_h - (overlap if row > 0 else 0)
    x1 = fitted_rect.x0 + (col + 1) * tile_w + (overlap if col < cols - 1 else 0)
    y1 = fitted_rect.y0 + (row + 1) * tile_h + (overlap if row < rows - 1 else 0)
    return fitz.Rect(max(fitted_rect.x0, x0), max(fitted_rect.y0, y0), min(fitted_rect.x1, x1), min(fitted_rect.y1, y1))


def _draw_cut_marks(page: fitz.Page, content: fitz.Rect, mark_len: float = 12, color=(0.0, 0.0, 0.0)) -> None:
    # Subtle outer border / page trim helpers. Keep visually quiet.
    opacity = 0.28
    lines = [
        ((content.x0 - mark_len, content.y0), (content.x0 - 2, content.y0)),
        ((content.x0, content.y0 - mark_len), (content.x0, content.y0 - 2)),
        ((content.x1 + 2, content.y0), (content.x1 + mark_len, content.y0)),
        ((content.x1, content.y0 - mark_len), (content.x1, content.y0 - 2)),
        ((content.x0 - mark_len, content.y1), (content.x0 - 2, content.y1)),
        ((content.x0, content.y1 + 2), (content.x0, content.y1 + mark_len)),
        ((content.x1 + 2, content.y1), (content.x1 + mark_len, content.y1)),
        ((content.x1, content.y1 + 2), (content.x1, content.y1 + mark_len)),
    ]
    for p0, p1 in lines:
        page.draw_line(p0, p1, color=color, width=0.45, stroke_opacity=opacity)
    page.draw_rect(content, color=color, width=0.3, dashes="[3 3] 0", stroke_opacity=opacity)


def _content_x(content: fitz.Rect, clip_canvas: fitz.Rect, canvas_x: float) -> float:
    return content.x0 + (canvas_x - clip_canvas.x0) * content.width / clip_canvas.width


def _content_y(content: fitz.Rect, clip_canvas: fitz.Rect, canvas_y: float) -> float:
    return content.y0 + (canvas_y - clip_canvas.y0) * content.height / clip_canvas.height


def _draw_x_box(page: fitz.Page, cx: float, cy: float, size: float, color=(0.0, 0.25, 0.95)) -> None:
    rect = fitz.Rect(cx - size / 2, cy - size / 2, cx + size / 2, cy + size / 2)
    page.draw_rect(rect, color=color, width=0.7)
    page.draw_line((rect.x0, rect.y0), (rect.x1, rect.y1), color=color, width=0.55)
    page.draw_line((rect.x0, rect.y1), (rect.x1, rect.y0), color=color, width=0.55)


def _line_outer_points(p0: tuple[float, float], p1: tuple[float, float], offset: float = 12) -> tuple[tuple[float, float], tuple[float, float]]:
    dx = p1[0] - p0[0]
    dy = p1[1] - p0[1]
    length = (dx * dx + dy * dy) ** 0.5 or 1
    return (
        (p0[0] - dx / length * offset, p0[1] - dy / length * offset),
        (p1[0] + dx / length * offset, p1[1] + dy / length * offset),
    )


def _draw_crop_line_with_end_boxes(page: fitz.Page, p0: tuple[float, float], p1: tuple[float, float], color=(0.9, 0.0, 0.0)) -> None:
    page.draw_line(p0, p1, color=color, width=0.9, dashes="[7 3] 0")
    start, end = _line_outer_points(p0, p1)
    _draw_x_box(page, start[0], start[1], 7, color)
    _draw_x_box(page, end[0], end[1], 7, color)


def _draw_alignment_end_boxes(page: fitz.Page, p0: tuple[float, float], p1: tuple[float, float], color=(0.9, 0.0, 0.0)) -> None:
    # Boxes only: use when this side must align to another page's crop line.
    start, end = _line_outer_points(p0, p1)
    _draw_x_box(page, start[0], start[1], 7, color)
    _draw_x_box(page, end[0], end[1], 7, color)


def _draw_alignment_extension(page: fitz.Page, p0: tuple[float, float], p1: tuple[float, float], color=(0.0, 0.25, 0.95)) -> None:
    # Alignment line stays outside image only.
    if abs(p0[0] - p1[0]) < 0.01 and abs(p0[1] - p1[1]) < 0.01:
        return
    page.draw_line(p0, p1, color=color, width=0.65, dashes="[4 3] 0")
    _draw_x_box(page, p0[0], p0[1], 7, color)
    _draw_x_box(page, p1[0], p1[1], 7, color)


def _draw_assembly_guides(
    page: fitz.Page,
    content: fitz.Rect,
    clip_canvas: fitz.Rect,
    base_canvas: fitz.Rect,
    col: int,
    row: int,
    cols: int,
    rows: int,
) -> None:
    """Draw only crop lines, outside alignment lines, and endpoint boxed-X marks."""
    crop = (0.9, 0.0, 0.0)
    align = (0.0, 0.25, 0.95)

    left_x = _content_x(content, clip_canvas, base_canvas.x0)
    right_x = _content_x(content, clip_canvas, base_canvas.x1)
    top_y = _content_y(content, clip_canvas, base_canvas.y0)
    bottom_y = _content_y(content, clip_canvas, base_canvas.y1)

    # Crop lines: may cover image. Boxes at line ends.
    if col > 0:
        _draw_crop_line_with_end_boxes(page, (left_x, content.y0), (left_x, content.y1), crop)
    if row > 0:
        _draw_crop_line_with_end_boxes(page, (content.x0, top_y), (content.x1, top_y), crop)

    # Matching sides that align to another page's crop line: boxes only, no line.
    if col < cols - 1:
        _draw_alignment_end_boxes(page, (right_x, content.y0), (right_x, content.y1), crop)
    if row < rows - 1:
        _draw_alignment_end_boxes(page, (content.x0, bottom_y), (content.x1, bottom_y), crop)


def _base_tile_rect(fitted_rect: fitz.Rect, col: int, row: int, cols: int, rows: int) -> fitz.Rect:
    tile_w = fitted_rect.width / cols
    tile_h = fitted_rect.height / rows
    return fitz.Rect(
        fitted_rect.x0 + col * tile_w,
        fitted_rect.y0 + row * tile_h,
        fitted_rect.x0 + (col + 1) * tile_w,
        fitted_rect.y0 + (row + 1) * tile_h,
    )


def generate_poster_pdf(input_path: str | Path, output_path: str | Path, options: PosterOptions) -> list[GeneratedPage]:
    if options.cols < 1 or options.rows < 1:
        raise ValueError("Rows/cols must be >= 1")
    if options.overlap_mm < 0 or options.margin_mm < 0:
        raise ValueError("Overlap/margin must be >= 0")

    src_doc, src_pno = load_source_as_pdf(input_path, options.page_index, options.dpi)
    src_page = src_doc[src_pno]
    src_rect = src_page.rect

    out = fitz.open()
    margin = mm(options.margin_mm)
    landscape = choose_landscape(src_rect, options.cols, options.rows, margin) if options.auto_landscape else options.landscape
    w, h = page_size(landscape)
    overlap = mm(options.overlap_mm)
    printable = fitz.Rect(margin, margin, w - margin, h - margin)

    # Virtual poster canvas uses non-overlapped printable area per tile.
    total_w = printable.width * options.cols
    total_h = printable.height * options.rows
    fitted = _source_fit_rect(src_rect, total_w, total_h)

    generated: list[GeneratedPage] = []
    for row in range(options.rows):
        for col in range(options.cols):
            page = out.new_page(width=w, height=h)
            clip_canvas = _tile_clip(fitted, col, row, options.cols, options.rows, overlap)
            visible_w = clip_canvas.width
            visible_h = clip_canvas.height
            scale = min(printable.width / visible_w, printable.height / visible_h)
            draw_w = visible_w * scale
            draw_h = visible_h * scale
            dest = fitz.Rect(
                (w - draw_w) / 2,
                (h - draw_h) / 2,
                (w + draw_w) / 2,
                (h + draw_h) / 2,
            )

            # Convert canvas clip into source-page coordinates.
            sx = src_rect.width / fitted.width
            sy = src_rect.height / fitted.height
            src_clip = fitz.Rect(
                src_rect.x0 + (clip_canvas.x0 - fitted.x0) * sx,
                src_rect.y0 + (clip_canvas.y0 - fitted.y0) * sy,
                src_rect.x0 + (clip_canvas.x1 - fitted.x0) * sx,
                src_rect.y0 + (clip_canvas.y1 - fitted.y0) * sy,
            )
            page.show_pdf_page(dest, src_doc, src_pno, clip=src_clip, keep_proportion=False)

            base_canvas = _base_tile_rect(fitted, col, row, options.cols, options.rows)
            if options.draw_cut_marks:
                _draw_cut_marks(page, dest)
            if options.draw_alignment_guides:
                _draw_assembly_guides(page, dest, clip_canvas, base_canvas, col, row, options.cols, options.rows)
            if options.draw_labels:
                label = f"{Path(input_path).name}  |  row {row + 1}/{options.rows}, col {col + 1}/{options.cols}  |  overlap {options.overlap_mm:g} mm"
                page.insert_text((margin, h - margin / 2), label, fontsize=8, color=(0.2, 0.2, 0.2))

            generated.append(GeneratedPage(col=col + 1, row=row + 1, clip=src_clip))

    out.save(output_path, garbage=4, deflate=True)
    out.close()
    src_doc.close()
    return generated


def parse_grid(value: str) -> tuple[int, int]:
    parts = value.lower().replace("×", "x").split("x")
    if len(parts) != 2:
        raise argparse.ArgumentTypeError("Grid must look like 2x3")
    cols, rows = (int(parts[0]), int(parts[1]))
    return cols, rows


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Split PDF/image into tiled A4 poster PDF")
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument("--grid", type=parse_grid, default=(2, 2), help="cols x rows, e.g. 2x3")
    parser.add_argument("--overlap-mm", type=float, default=10)
    parser.add_argument("--margin-mm", type=float, default=8)
    parser.add_argument("--dpi", type=int, default=200, help="DPI used for image physical size")
    parser.add_argument("--landscape", action="store_true")
    parser.add_argument("--auto-orientation", action="store_true", help="Automatically choose A4 portrait/landscape")
    parser.add_argument("--no-cut-marks", action="store_true")
    parser.add_argument("--no-alignment-guides", action="store_true")
    parser.add_argument("--labels", action="store_true")
    parser.add_argument("--page", type=int, default=1, help="PDF page number, 1-based")
    args = parser.parse_args(argv)

    cols, rows = args.grid
    options = PosterOptions(
        cols=cols,
        rows=rows,
        overlap_mm=args.overlap_mm,
        margin_mm=args.margin_mm,
        dpi=args.dpi,
        landscape=args.landscape,
        draw_cut_marks=not args.no_cut_marks,
        draw_alignment_guides=not args.no_alignment_guides,
        draw_labels=args.labels,
        page_index=args.page - 1,
        auto_landscape=args.auto_orientation,
    )
    pages = generate_poster_pdf(args.input, args.output, options)
    print(f"Generated {len(pages)} A4 pages: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
