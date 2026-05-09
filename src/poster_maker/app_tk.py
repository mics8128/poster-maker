from __future__ import annotations

import queue
import sys
import threading
from pathlib import Path
from tkinter import BooleanVar, DoubleVar, IntVar, StringVar, Tk, filedialog, messagebox, ttk

from . import __version__
from .core import PosterOptions, generate_poster_pdf, load_source_as_pdf, mm, page_size, poster_canvas_size, resolve_layout


def pt_to_cm(value: float) -> float:
    return value / 72 * 2.54


class PosterMakerTk:
    def __init__(self) -> None:
        self.root = Tk()
        self.root.title(f"Poster Maker v{__version__}")
        self.root.geometry("920x640")
        self.jobs: queue.Queue[tuple[str, str]] = queue.Queue()

        self.input_path = StringVar()
        self.output_path = StringVar()
        self.grid = StringVar(value="3x2 / 2x3")
        self.best_layout = BooleanVar(value=True)
        self.cols = IntVar(value=3)
        self.rows = IntVar(value=2)
        self.overlap = DoubleVar(value=10)
        self.margin = DoubleVar(value=8)
        self.dpi = IntVar(value=200)
        self.page = IntVar(value=1)
        self.landscape = BooleanVar(value=False)
        self.cut_marks = BooleanVar(value=True)
        self.guides = BooleanVar(value=True)
        self.labels = BooleanVar(value=False)

        self._build_ui()
        self._wire_updates()
        self.update_preview()
        self.root.after(150, self._poll_jobs)

    def _build_ui(self) -> None:
        outer = ttk.Frame(self.root, padding=12)
        outer.pack(fill="both", expand=True)
        outer.columnconfigure(1, weight=1)
        outer.rowconfigure(0, weight=1)

        left = ttk.Frame(outer)
        left.grid(row=0, column=0, sticky="nsw", padx=(0, 12))
        right = ttk.Frame(outer)
        right.grid(row=0, column=1, sticky="nsew")
        right.columnconfigure(0, weight=1)
        right.rowconfigure(0, weight=1)

        ttk.Label(left, text="來源").grid(row=0, column=0, sticky="w")
        ttk.Entry(left, textvariable=self.input_path, width=44).grid(row=1, column=0, sticky="ew")
        ttk.Button(left, text="選擇…", command=self.pick_input).grid(row=1, column=1, padx=(6, 0))

        ttk.Label(left, text="輸出 PDF").grid(row=2, column=0, sticky="w", pady=(10, 0))
        ttk.Entry(left, textvariable=self.output_path, width=44).grid(row=3, column=0, sticky="ew")
        ttk.Button(left, text="儲存…", command=self.pick_output).grid(row=3, column=1, padx=(6, 0))

        ttk.Label(left, text="A4 張數").grid(row=4, column=0, sticky="w", pady=(10, 0))
        grid_combo = ttk.Combobox(
            left,
            textvariable=self.grid,
            values=["2x1", "2x2", "3x2 / 2x3", "3x3", "4x3 / 3x4", "4x4", "Custom"],
            state="readonly",
            width=18,
        )
        grid_combo.grid(row=5, column=0, sticky="w")
        grid_combo.bind("<<ComboboxSelected>>", lambda _e: self.grid_changed())
        ttk.Checkbutton(left, text="最佳擺放", variable=self.best_layout, command=self.update_preview).grid(row=6, column=0, sticky="w", pady=(6, 0))

        advanced = ttk.LabelFrame(left, text="進階")
        advanced.grid(row=7, column=0, columnspan=2, sticky="ew", pady=(14, 0))
        self.advanced_visible = BooleanVar(value=False)
        ttk.Checkbutton(advanced, text="顯示進階選項", variable=self.advanced_visible, command=self.toggle_advanced).grid(row=0, column=0, sticky="w")
        self.advanced_body = ttk.Frame(advanced)
        self.advanced_body.grid(row=1, column=0, sticky="ew")

        row = 0
        for label, var, width in [
            ("欄", self.cols, 6),
            ("列", self.rows, 6),
            ("重疊 mm", self.overlap, 8),
            ("邊界 mm", self.margin, 8),
            ("圖片 DPI", self.dpi, 8),
            ("PDF 頁碼", self.page, 8),
        ]:
            ttk.Label(self.advanced_body, text=label).grid(row=row, column=0, sticky="w", pady=2)
            ttk.Entry(self.advanced_body, textvariable=var, width=width).grid(row=row, column=1, sticky="w", pady=2)
            row += 1
        ttk.Checkbutton(self.advanced_body, text="強制 A4 橫向（關閉最佳擺放時）", variable=self.landscape, command=self.update_preview).grid(row=row, column=0, columnspan=2, sticky="w"); row += 1
        ttk.Checkbutton(self.advanced_body, text="淡黑外框裁切輔助", variable=self.cut_marks).grid(row=row, column=0, columnspan=2, sticky="w"); row += 1
        ttk.Checkbutton(self.advanced_body, text="紅色裁切線與 X 框", variable=self.guides).grid(row=row, column=0, columnspan=2, sticky="w"); row += 1
        ttk.Checkbutton(self.advanced_body, text="頁面文字標籤", variable=self.labels).grid(row=row, column=0, columnspan=2, sticky="w")
        self.toggle_advanced()

        ttk.Button(left, text="產生海報 PDF", command=self.generate).grid(row=8, column=0, columnspan=2, sticky="ew", pady=(14, 8))
        self.status = ttk.Label(left, text="選 PDF 或圖片，按產生。", wraplength=360)
        self.status.grid(row=9, column=0, columnspan=2, sticky="ew")
        ttk.Label(left, text=f"Poster Maker v{__version__}", foreground="#777").grid(row=10, column=0, columnspan=2, sticky="sew", pady=(30, 0))

        self.canvas = __import__("tkinter").Canvas(right, bg="#f6f6f6", highlightthickness=1, highlightbackground="#ccc")
        self.canvas.grid(row=0, column=0, sticky="nsew")
        self.canvas.bind("<Configure>", lambda _e: self.update_preview())

    def _wire_updates(self) -> None:
        for var in (self.input_path, self.output_path, self.cols, self.rows, self.overlap, self.margin, self.dpi, self.page):
            var.trace_add("write", lambda *_args: self.update_preview())

    def toggle_advanced(self) -> None:
        if self.advanced_visible.get():
            self.advanced_body.grid()
        else:
            self.advanced_body.grid_remove()

    def grid_changed(self) -> None:
        text = self.grid.get()
        if text != "Custom":
            first = text.split("/")[0].strip()
            c, r = first.split("x")
            self.cols.set(int(c))
            self.rows.set(int(r))
        self.update_preview()

    def current_options(self) -> PosterOptions:
        return PosterOptions(
            cols=max(1, self.cols.get()),
            rows=max(1, self.rows.get()),
            overlap_mm=max(0, self.overlap.get()),
            margin_mm=max(0, self.margin.get()),
            dpi=max(72, self.dpi.get()),
            landscape=self.landscape.get(),
            draw_cut_marks=self.cut_marks.get(),
            draw_alignment_guides=self.guides.get(),
            draw_labels=self.labels.get(),
            page_index=max(0, self.page.get() - 1),
            auto_landscape=self.best_layout.get(),
            auto_layout=self.best_layout.get(),
        )

    def update_preview(self) -> None:
        self.canvas.delete("all")
        path = self.input_path.get().strip()
        if not path:
            self.canvas.create_text(20, 20, text="預覽：請先選來源", anchor="nw")
            return
        try:
            opts = self.current_options()
            doc, pno = load_source_as_pdf(path, opts.page_index, opts.dpi)
            src_rect = doc[pno].rect
            layout = resolve_layout(src_rect, opts)
            w_pt, h_pt = page_size(layout.landscape)
            poster_w = w_pt * layout.cols
            poster_h = h_pt * layout.rows
            cw = max(320, self.canvas.winfo_width() - 24)
            ch = max(300, self.canvas.winfo_height() - 24)
            scale = min(cw / poster_w, ch / poster_h)
            ox = (self.canvas.winfo_width() - poster_w * scale) / 2
            oy = (self.canvas.winfo_height() - poster_h * scale) / 2

            margin_pt = mm(opts.margin_mm)
            printable = type(src_rect)(margin_pt, margin_pt, w_pt - margin_pt, h_pt - margin_pt)
            _base_w, _base_h, printable_w, printable_h = poster_canvas_size(printable, layout.cols, layout.rows, mm(opts.overlap_mm))
            fit_scale = min(printable_w / src_rect.width, printable_h / src_rect.height)
            fitted_w_pt = src_rect.width * fit_scale
            fitted_h_pt = src_rect.height * fit_scale
            fit_w = fitted_w_pt * scale
            fit_h = fitted_h_pt * scale
            fit_x = ox + (poster_w * scale - fit_w) / 2
            fit_y = oy + (poster_h * scale - fit_h) / 2
            self.canvas.create_rectangle(fit_x, fit_y, fit_x + fit_w, fit_y + fit_h, fill="#ececec", outline="")

            for r in range(layout.rows):
                for c in range(layout.cols):
                    x = ox + c * w_pt * scale
                    y = oy + r * h_pt * scale
                    self.canvas.create_rectangle(x, y, x + w_pt * scale, y + h_pt * scale, outline="#111")
                    m = margin_pt * scale
                    self.canvas.create_rectangle(x + m, y + m, x + w_pt * scale - m, y + h_pt * scale - m, outline="#999", dash=(2, 2))
            for c in range(1, layout.cols):
                x = ox + c * w_pt * scale
                self.canvas.create_line(x, oy, x, oy + poster_h * scale, fill="red", dash=(5, 3))
            for r in range(1, layout.rows):
                y = oy + r * h_pt * scale
                self.canvas.create_line(ox, y, ox + poster_w * scale, y, fill="red", dash=(5, 3))

            paper_w_cm = pt_to_cm(poster_w)
            paper_h_cm = pt_to_cm(poster_h)
            image_w_cm = pt_to_cm(fitted_w_pt)
            image_h_cm = pt_to_cm(fitted_h_pt)
            self.status.configure(
                text=(
                    f"最佳輸出：{layout.cols}x{layout.rows} A4，{'橫向' if layout.landscape else '直向'}\n"
                    f"成品圖面：約 {image_w_cm:.1f} × {image_h_cm:.1f} cm\n"
                    f"A4總外框：約 {paper_w_cm:.1f} × {paper_h_cm:.1f} cm\n"
                    f"重疊 {opts.overlap_mm:g}mm，邊界 {opts.margin_mm:g}mm"
                )
            )
            doc.close()
        except Exception as exc:
            self.canvas.create_text(20, 20, text=f"預覽失敗：{exc}", anchor="nw")

    def pick_input(self) -> None:
        path = filedialog.askopenfilename(filetypes=[("PDF/Images", "*.pdf *.png *.jpg *.jpeg *.webp *.bmp *.tif *.tiff")])
        if not path:
            return
        self.input_path.set(path)
        if not self.output_path.get():
            p = Path(path)
            self.output_path.set(str(p.with_name(f"{p.stem}-poster.pdf")))

    def pick_output(self) -> None:
        path = filedialog.asksaveasfilename(defaultextension=".pdf", filetypes=[("PDF", "*.pdf")], initialfile=self.output_path.get())
        if path:
            self.output_path.set(path)

    def generate(self) -> None:
        input_path = self.input_path.get().strip()
        output_path = self.output_path.get().strip()
        if not input_path or not output_path:
            messagebox.showwarning("缺少路徑", "請選擇來源與輸出 PDF。")
            return
        opts = self.current_options()
        self.status.configure(text="產生中…")

        def work() -> None:
            try:
                pages = generate_poster_pdf(input_path, output_path, opts)
                self.jobs.put(("ok", f"完成：{len(pages)} 頁 A4 → {output_path}"))
            except Exception as exc:
                self.jobs.put(("err", str(exc)))

        threading.Thread(target=work, daemon=True).start()

    def _poll_jobs(self) -> None:
        try:
            kind, message = self.jobs.get_nowait()
        except queue.Empty:
            pass
        else:
            self.status.configure(text=message)
            if kind == "ok":
                messagebox.showinfo("完成", message)
            else:
                messagebox.showerror("產生失敗", message)
        self.root.after(150, self._poll_jobs)


def main() -> int:
    app = PosterMakerTk()
    app.root.mainloop()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
