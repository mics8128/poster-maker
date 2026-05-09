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
    QScrollArea,
)

from .core import PosterOptions, choose_landscape, generate_poster_pdf, load_source_as_pdf, mm, page_size


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
        self.setWindowTitle("Poster Maker")
        self.setMinimumWidth(960)
        self.worker: Worker | None = None

        self.input_edit = QLineEdit()
        self.output_edit = QLineEdit()
        self.grid_combo = QComboBox()
        self.grid_combo.addItems(["2x1", "2x2", "2x3", "3x2", "3x3", "4x3", "4x4", "Custom"])
        self.cols_spin = QSpinBox()
        self.cols_spin.setRange(1, 12)
        self.cols_spin.setValue(2)
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
        self.auto_orientation_check = QCheckBox("自動偵測 A4 直/橫向")
        self.auto_orientation_check.setChecked(True)
        self.landscape_check = QCheckBox("A4 橫向")
        self.cut_marks_check = QCheckBox("輔助裁切線")
        self.cut_marks_check.setChecked(True)
        self.alignment_check = QCheckBox("拼貼輔助對齊線")
        self.alignment_check.setChecked(True)
        self.labels_check = QCheckBox("頁面標籤")
        self.labels_check.setChecked(False)
        self.status_label = QLabel("選 PDF 或圖片，輸出為多頁 A4 PDF")
        self.preview_label = QLabel("預覽")
        self.preview_label.setAlignment(Qt.AlignCenter)
        self.preview_label.setMinimumSize(420, 520)
        self.preview_label.setStyleSheet("background:#f6f6f6; border:1px solid #ccc;")
        self.generate_button = QPushButton("產生海報 PDF")

        input_button = QPushButton("選擇…")
        output_button = QPushButton("儲存…")
        input_button.clicked.connect(self.pick_input)
        output_button.clicked.connect(self.pick_output)
        self.generate_button.clicked.connect(self.generate)
        self.grid_combo.currentTextChanged.connect(self.grid_changed)
        self.input_edit.textChanged.connect(self.update_preview)
        self.cols_spin.valueChanged.connect(self.update_preview)
        self.rows_spin.valueChanged.connect(self.update_preview)
        self.overlap_spin.valueChanged.connect(self.update_preview)
        self.margin_spin.valueChanged.connect(self.update_preview)
        self.page_spin.valueChanged.connect(self.update_preview)
        self.landscape_check.toggled.connect(self.update_preview)
        self.auto_orientation_check.toggled.connect(self.update_preview)

        input_row = QHBoxLayout()
        input_row.addWidget(self.input_edit)
        input_row.addWidget(input_button)
        output_row = QHBoxLayout()
        output_row.addWidget(self.output_edit)
        output_row.addWidget(output_button)
        custom_row = QHBoxLayout()
        custom_row.addWidget(QLabel("欄 cols"))
        custom_row.addWidget(self.cols_spin)
        custom_row.addWidget(QLabel("列 rows"))
        custom_row.addWidget(self.rows_spin)

        form = QFormLayout()
        form.addRow("來源 PDF/圖片", input_row)
        form.addRow("輸出 PDF", output_row)
        form.addRow("A4 組合", self.grid_combo)
        form.addRow("自訂組合", custom_row)
        form.addRow("重疊區", self.overlap_spin)
        form.addRow("邊界", self.margin_spin)
        form.addRow("圖片 DPI", self.dpi_spin)
        form.addRow("PDF 頁碼", self.page_spin)
        form.addRow("選項", self.auto_orientation_check)
        form.addRow("", self.landscape_check)
        form.addRow("", self.cut_marks_check)
        form.addRow("", self.alignment_check)
        form.addRow("", self.labels_check)

        left_layout = QVBoxLayout()
        left_layout.addLayout(form)
        left_layout.addWidget(self.generate_button)
        left_layout.addWidget(self.status_label)
        preview_scroll = QScrollArea()
        preview_scroll.setWidgetResizable(True)
        preview_scroll.setWidget(self.preview_label)
        layout = QHBoxLayout()
        left = QWidget()
        left.setLayout(left_layout)
        layout.addWidget(left, 0)
        layout.addWidget(preview_scroll, 1)
        container = QWidget()
        container.setLayout(layout)
        self.setCentralWidget(container)
        self.grid_changed(self.grid_combo.currentText())
        self.update_preview()

    def grid_changed(self, text: str) -> None:
        custom = text == "Custom"
        self.cols_spin.setEnabled(custom)
        self.rows_spin.setEnabled(custom)
        if not custom:
            cols, rows = text.split("x")
            self.cols_spin.setValue(int(cols))
            self.rows_spin.setValue(int(rows))


    def update_preview(self) -> None:
        path = self.input_edit.text().strip()
        if not path:
            self.preview_label.setText("預覽：請先選來源")
            return
        try:
            src_doc, pno = load_source_as_pdf(path, self.page_spin.value() - 1, self.dpi_spin.value())
            src_rect = src_doc[pno].rect
            cols = self.cols_spin.value()
            rows = self.rows_spin.value()
            margin_pt = mm(self.margin_spin.value())
            landscape = choose_landscape(src_rect, cols, rows, margin_pt) if self.auto_orientation_check.isChecked() else self.landscape_check.isChecked()
            w_pt, h_pt = page_size(landscape)
            poster_w = w_pt * cols
            poster_h = h_pt * rows
            max_w, max_h = 760, 620
            scale = min(max_w / poster_w, max_h / poster_h)
            img_w = max(260, int(poster_w * scale))
            img_h = max(260, int(poster_h * scale))
            image = QImage(img_w, img_h, QImage.Format_ARGB32)
            image.fill(0xFFFFFFFF)
            painter = QPainter(image)
            painter.setRenderHint(QPainter.Antialiasing)

            margin = margin_pt * scale
            a4_w = w_pt * scale
            a4_h = h_pt * scale
            pen_page = QPen(Qt.black, 1)
            pen_cut = QPen(Qt.red, 1, Qt.DashLine)
            pen_align = QPen(Qt.blue, 1, Qt.DashLine)
            pen_overlap = QPen(Qt.darkCyan, 1, Qt.DotLine)

            # Source preview fitted across printable poster area.
            printable_w = (w_pt - 2 * margin_pt) * cols
            printable_h = (h_pt - 2 * margin_pt) * rows
            fit_scale = min(printable_w / src_rect.width, printable_h / src_rect.height)
            fit_w = src_rect.width * fit_scale * scale
            fit_h = src_rect.height * fit_scale * scale
            fit_x = (img_w - fit_w) / 2
            fit_y = (img_h - fit_h) / 2
            painter.fillRect(int(fit_x), int(fit_y), int(fit_w), int(fit_h), 0xFFECECEC)

            for r in range(rows):
                for c in range(cols):
                    x = c * a4_w
                    y = r * a4_h
                    painter.setPen(pen_page)
                    painter.drawRect(int(x), int(y), int(a4_w), int(a4_h))
                    painter.setPen(pen_cut)
                    painter.drawRect(int(x + margin), int(y + margin), int(a4_w - 2 * margin), int(a4_h - 2 * margin))
                    painter.drawText(int(x + 8), int(y + 18), f"{r+1},{c+1}")

            # Grid boundaries for final poster.
            painter.setPen(pen_align)
            for c in range(1, cols):
                x = c * a4_w
                painter.drawLine(int(x), 0, int(x), img_h)
            for r in range(1, rows):
                y = r * a4_h
                painter.drawLine(0, int(y), img_w, int(y))

            # Overlap guide bands near internal seams.
            overlap = mm(self.overlap_spin.value()) * scale
            painter.setPen(pen_overlap)
            for c in range(1, cols):
                x = c * a4_w
                painter.drawLine(int(x - overlap), 0, int(x - overlap), img_h)
                painter.drawLine(int(x + overlap), 0, int(x + overlap), img_h)
            for r in range(1, rows):
                y = r * a4_h
                painter.drawLine(0, int(y - overlap), img_w, int(y - overlap))
                painter.drawLine(0, int(y + overlap), img_w, int(y + overlap))

            painter.end()
            self.preview_label.setPixmap(QPixmap.fromImage(image))
            self.status_label.setText(f"預覽：{cols}x{rows} A4，{'橫向' if landscape else '直向'}；紅=裁切線，藍=對齊線")
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

        options = PosterOptions(
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
            auto_landscape=self.auto_orientation_check.isChecked(),
        )
        self.generate_button.setEnabled(False)
        self.status_label.setText("產生中…")
        self.worker = Worker(input_path, output_path, options)
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
