from __future__ import annotations

import sys
from pathlib import Path

from PySide6.QtCore import Qt, QThread, Signal
from PySide6.QtGui import QImage, QPainter, QPen, QPixmap
from PySide6.QtWidgets import (
    QApplication,
    QCheckBox,
    QComboBox,
    QFileDialog,
    QFormLayout,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMainWindow,
    QMessageBox,
    QPushButton,
    QSpinBox,
    QDoubleSpinBox,
    QVBoxLayout,
    QWidget,
)

from . import __version__
from .core import PosterOptions, generate_poster_pdf, load_source_as_pdf, mm, page_size, poster_canvas_size, resolve_layout


def pt_to_cm(value: float) -> float:
    return value / 72 * 2.54


class Worker(QThread):
    finished_ok = Signal(str, int)
    failed = Signal(str)

    def __init__(self, input_path: str, output_path: str, options: PosterOptions):
        super().__init__()
        self.input_path = input_path
        self.output_path = output_path
        self.options = options

    def run(self) -> None:
        try:
            pages = generate_poster_pdf(self.input_path, self.output_path, self.options)
            self.finished_ok.emit(self.output_path, len(pages))
        except Exception as exc:  # GUI boundary
            self.failed.emit(str(exc))


class MainWindow(QMainWindow):
    def __init__(self) -> None:
        super().__init__()
        self.setWindowTitle(f"Poster Maker v{__version__}")
        self.resize(980, 700)
        self.worker: Worker | None = None

        self.input_edit = QLineEdit()
        self.output_edit = QLineEdit()
        self.grid_combo = QComboBox()
        self.grid_combo.addItems(["2x1", "2x2", "3x2 / 2x3", "3x3", "4x3 / 3x4", "4x4", "Custom"])
        self.grid_combo.setCurrentText("3x2 / 2x3")
        self.cols_spin = QSpinBox()
        self.cols_spin.setRange(1, 12)
        self.cols_spin.setValue(3)
        self.rows_spin = QSpinBox()
        self.rows_spin.setRange(1, 12)
        self.rows_spin.setValue(2)
        self.overlap_spin = QDoubleSpinBox()
        self.overlap_spin.setRange(0, 50)
        self.overlap_spin.setValue(10)
        self.overlap_spin.setSuffix(" mm")
        self.margin_spin = QDoubleSpinBox()
        self.margin_spin.setRange(0, 30)
        self.margin_spin.setValue(8)
        self.margin_spin.setSuffix(" mm")
        self.dpi_spin = QSpinBox()
        self.dpi_spin.setRange(72, 600)
        self.dpi_spin.setValue(200)
        self.page_spin = QSpinBox()
        self.page_spin.setRange(1, 9999)
        self.page_spin.setValue(1)
        self.best_layout_check = QCheckBox("最佳擺放")
        self.best_layout_check.setChecked(True)
        self.landscape_check = QCheckBox("強制 A4 橫向（關閉最佳擺放時使用）")
        self.cut_marks_check = QCheckBox("淡黑外框裁切輔助")
        self.cut_marks_check.setChecked(True)
        self.alignment_check = QCheckBox("紅色裁切線與 X 框")
        self.alignment_check.setChecked(True)
        self.labels_check = QCheckBox("頁面文字標籤")
        self.labels_check.setChecked(False)
        self.advanced_box = QGroupBox("進階")
        self.advanced_box.setCheckable(True)
        self.advanced_box.setChecked(False)
        self.status_label = QLabel("選 PDF 或圖片，按產生。預設使用最佳擺放與建議參數。")
        self.status_label.setWordWrap(True)
        self.version_label = QLabel(f"Poster Maker v{__version__}")
        self.version_label.setAlignment(Qt.AlignRight)
        self.version_label.setStyleSheet("color:#777; font-size:11px;")
        self.preview_label = QLabel("預覽：請先選來源")
        self.preview_label.setAlignment(Qt.AlignCenter)
        self.preview_label.setSizePolicy(self.preview_label.sizePolicy().horizontalPolicy(), self.preview_label.sizePolicy().verticalPolicy())
        self.preview_label.setStyleSheet("background:#f6f6f6; border:1px solid #ccc;")
        self.generate_button = QPushButton("產生海報 PDF")

        input_button = QPushButton("選擇…")
        output_button = QPushButton("儲存…")
        input_button.clicked.connect(self.pick_input)
        output_button.clicked.connect(self.pick_output)
        self.generate_button.clicked.connect(self.generate)
        self.grid_combo.currentTextChanged.connect(self.grid_changed)
        self.best_layout_check.toggled.connect(self.update_preview)

        for widget in (
            self.input_edit,
            self.cols_spin,
            self.rows_spin,
            self.overlap_spin,
            self.margin_spin,
            self.dpi_spin,
            self.page_spin,
            self.landscape_check,
            self.cut_marks_check,
            self.alignment_check,
            self.labels_check,
        ):
            signal = getattr(widget, "textChanged", None) or getattr(widget, "valueChanged", None) or getattr(widget, "toggled", None)
            signal.connect(self.update_preview)

        input_row = QHBoxLayout()
        input_row.addWidget(self.input_edit)
        input_row.addWidget(input_button)
        output_row = QHBoxLayout()
        output_row.addWidget(self.output_edit)
        output_row.addWidget(output_button)

        main_form = QFormLayout()
        main_form.addRow("來源", input_row)
        main_form.addRow("輸出", output_row)
        main_form.addRow("A4 張數", self.grid_combo)
        main_form.addRow("", self.best_layout_check)

        custom_row = QHBoxLayout()
        custom_row.addWidget(QLabel("欄"))
        custom_row.addWidget(self.cols_spin)
        custom_row.addWidget(QLabel("列"))
        custom_row.addWidget(self.rows_spin)

        advanced_form = QFormLayout()
        advanced_form.addRow("自訂欄列", custom_row)
        advanced_form.addRow("重疊區", self.overlap_spin)
        advanced_form.addRow("邊界", self.margin_spin)
        advanced_form.addRow("圖片 DPI", self.dpi_spin)
        advanced_form.addRow("PDF 頁碼", self.page_spin)
        advanced_form.addRow("", self.landscape_check)
        advanced_form.addRow("", self.cut_marks_check)
        advanced_form.addRow("", self.alignment_check)
        advanced_form.addRow("", self.labels_check)
        self.advanced_box.setLayout(advanced_form)
        self.advanced_box.toggled.connect(self._advanced_toggled)
        self._advanced_toggled(False)

        left_layout = QVBoxLayout()
        left_layout.addLayout(main_form)
        left_layout.addWidget(self.advanced_box)
        left_layout.addWidget(self.generate_button)
        left_layout.addWidget(self.status_label)
        left_layout.addStretch(1)
        left_layout.addWidget(self.version_label)

        layout = QHBoxLayout()
        left = QWidget()
        left.setLayout(left_layout)
        layout.addWidget(left, 0)
        layout.addWidget(self.preview_label, 1)
        container = QWidget()
        container.setLayout(layout)
        self.setCentralWidget(container)
        self.grid_changed(self.grid_combo.currentText())
        self.update_preview()

    def _advanced_toggled(self, checked: bool) -> None:
        for child in self.advanced_box.findChildren(QWidget):
            child.setVisible(checked)

    def grid_changed(self, text: str) -> None:
        custom = text == "Custom"
        self.cols_spin.setEnabled(custom)
        self.rows_spin.setEnabled(custom)
        if not custom:
            first = text.split("/")[0].strip()
            cols, rows = first.split("x")
            self.cols_spin.setValue(int(cols))
            self.rows_spin.setValue(int(rows))
        self.update_preview()

    def current_options(self) -> PosterOptions:
        return PosterOptions(
            cols=self.cols_spin.value(),
            rows=self.rows_spin.value(),
            overlap_mm=self.overlap_spin.value(),
            margin_mm=self.margin_spin.value(),
            dpi=self.dpi_spin.value(),
            landscape=self.landscape_check.isChecked(),
            draw_cut_marks=self.cut_marks_check.isChecked(),
            draw_alignment_guides=self.alignment_check.isChecked(),
            draw_labels=self.labels_check.isChecked(),
            page_index=self.page_spin.value() - 1,
            auto_landscape=self.best_layout_check.isChecked(),
            auto_layout=self.best_layout_check.isChecked(),
        )

    def resizeEvent(self, event) -> None:  # noqa: N802 - Qt API
        super().resizeEvent(event)
        self.update_preview()

    def update_preview(self) -> None:
        path = self.input_edit.text().strip()
        if not path:
            self.preview_label.setText("預覽：請先選來源")
            return
        try:
            options = self.current_options()
            src_doc, pno = load_source_as_pdf(path, options.page_index, options.dpi)
            src_rect = src_doc[pno].rect
            layout = resolve_layout(src_rect, options)
            w_pt, h_pt = page_size(layout.landscape)
            poster_w = w_pt * layout.cols
            poster_h = h_pt * layout.rows

            available_w = max(360, self.preview_label.width() - 24)
            available_h = max(320, self.preview_label.height() - 24)
            scale = min(available_w / poster_w, available_h / poster_h)
            img_w = max(240, int(poster_w * scale))
            img_h = max(240, int(poster_h * scale))
            image = QImage(img_w, img_h, QImage.Format_ARGB32)
            image.fill(0xFFFFFFFF)
            painter = QPainter(image)
            painter.setRenderHint(QPainter.Antialiasing)

            margin_pt = mm(options.margin_mm)
            margin = margin_pt * scale
            a4_w = w_pt * scale
            a4_h = h_pt * scale
            pen_page = QPen(Qt.black, 1)
            pen_cut = QPen(Qt.red, 1, Qt.DashLine)
            pen_outer = QPen(Qt.black, 1, Qt.DotLine)
            pen_outer.setColor(Qt.gray)
            pen_overlap = QPen(Qt.darkCyan, 1, Qt.DotLine)

            printable = type(src_rect)(margin_pt, margin_pt, w_pt - margin_pt, h_pt - margin_pt)
            _base_w, _base_h, printable_w, printable_h = poster_canvas_size(printable, layout.cols, layout.rows, mm(options.overlap_mm))
            fit_scale = min(printable_w / src_rect.width, printable_h / src_rect.height)
            fitted_w_pt = src_rect.width * fit_scale
            fitted_h_pt = src_rect.height * fit_scale
            fit_w = fitted_w_pt * scale
            fit_h = fitted_h_pt * scale
            fit_x = (img_w - fit_w) / 2
            fit_y = (img_h - fit_h) / 2
            painter.fillRect(int(fit_x), int(fit_y), int(fit_w), int(fit_h), 0xFFECECEC)

            for r in range(layout.rows):
                for c in range(layout.cols):
                    x = c * a4_w
                    y = r * a4_h
                    painter.setPen(pen_page)
                    painter.drawRect(int(x), int(y), int(a4_w), int(a4_h))
                    painter.setPen(pen_outer)
                    painter.drawRect(int(x + margin), int(y + margin), int(a4_w - 2 * margin), int(a4_h - 2 * margin))

            painter.setPen(pen_cut)
            for c in range(1, layout.cols):
                x = c * a4_w
                painter.drawLine(int(x), 0, int(x), img_h)
            for r in range(1, layout.rows):
                y = r * a4_h
                painter.drawLine(0, int(y), img_w, int(y))

            overlap = mm(options.overlap_mm) * scale
            painter.setPen(pen_overlap)
            for c in range(1, layout.cols):
                x = c * a4_w
                painter.drawLine(int(x - overlap), 0, int(x - overlap), img_h)
                painter.drawLine(int(x + overlap), 0, int(x + overlap), img_h)
            for r in range(1, layout.rows):
                y = r * a4_h
                painter.drawLine(0, int(y - overlap), img_w, int(y - overlap))
                painter.drawLine(0, int(y + overlap), img_w, int(y + overlap))

            painter.end()
            self.preview_label.setPixmap(QPixmap.fromImage(image))
            paper_w_cm = pt_to_cm(poster_w)
            paper_h_cm = pt_to_cm(poster_h)
            image_w_cm = pt_to_cm(fitted_w_pt)
            image_h_cm = pt_to_cm(fitted_h_pt)
            self.status_label.setText(
                f"最佳輸出：{layout.cols}x{layout.rows} A4，{'橫向' if layout.landscape else '直向'}\n"
                f"成品圖面：約 {image_w_cm:.1f} × {image_h_cm:.1f} cm\n"
                f"A4總外框：約 {paper_w_cm:.1f} × {paper_h_cm:.1f} cm\n"
                f"重疊 {options.overlap_mm:g}mm，邊界 {options.margin_mm:g}mm"
            )
            src_doc.close()
        except Exception as exc:
            self.preview_label.setText(f"預覽失敗：{exc}")

    def pick_input(self) -> None:
        path, _ = QFileDialog.getOpenFileName(
            self,
            "選擇來源",
            "",
            "PDF/Images (*.pdf *.png *.jpg *.jpeg *.webp *.bmp *.tif *.tiff)",
        )
        if not path:
            return
        self.input_edit.setText(path)
        if not self.output_edit.text():
            p = Path(path)
            self.output_edit.setText(str(p.with_name(f"{p.stem}-poster.pdf")))

    def pick_output(self) -> None:
        path, _ = QFileDialog.getSaveFileName(self, "儲存輸出 PDF", self.output_edit.text(), "PDF (*.pdf)")
        if not path:
            return
        if not path.lower().endswith(".pdf"):
            path += ".pdf"
        self.output_edit.setText(path)

    def generate(self) -> None:
        input_path = self.input_edit.text().strip()
        output_path = self.output_edit.text().strip()
        if not input_path or not output_path:
            QMessageBox.warning(self, "缺少路徑", "請選擇來源與輸出 PDF。")
            return

        self.generate_button.setEnabled(False)
        self.status_label.setText("產生中…")
        self.worker = Worker(input_path, output_path, self.current_options())
        self.worker.finished_ok.connect(self.done)
        self.worker.failed.connect(self.fail)
        self.worker.start()

    def done(self, output_path: str, count: int) -> None:
        self.generate_button.setEnabled(True)
        self.status_label.setText(f"完成：{count} 頁 A4 → {output_path}")
        QMessageBox.information(self, "完成", f"已產生 {count} 頁 A4 PDF。")

    def fail(self, message: str) -> None:
        self.generate_button.setEnabled(True)
        self.status_label.setText("失敗")
        QMessageBox.critical(self, "產生失敗", message)


def main() -> int:
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
