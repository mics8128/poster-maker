#!/usr/bin/env python3
from __future__ import annotations

import shutil
from pathlib import Path

ROOTS = [Path("dist/PosterMaker/_internal"), Path("dist/PosterMaker.app/Contents/Frameworks")]

REMOVE_DIRS = [
    "PySide6/Qt/translations",
    "PySide6/Qt/lib/QtPdf.framework",
    "PySide6/Qt/lib/QtQml.framework",
    "PySide6/Qt/lib/QtQmlMeta.framework",
    "PySide6/Qt/lib/QtQmlModels.framework",
    "PySide6/Qt/lib/QtQmlWorkerScript.framework",
    "PySide6/Qt/lib/QtQuick.framework",
    "PySide6/Qt/lib/QtVirtualKeyboard.framework",
    "PySide6/Qt/lib/QtVirtualKeyboardQml.framework",
]

REMOVE_GLOBS = [
    "QtOpenGL", "QtPdf", "QtQml", "QtQmlMeta", "QtQmlModels", "QtQmlWorkerScript", "QtQuick", "QtSvg", "QtVirtualKeyboard", "QtVirtualKeyboardQml",
    "PySide6/Qt/plugins/generic/*",
    "PySide6/Qt/plugins/iconengines/*",
    "PySide6/Qt/plugins/imageformats/libqgif*",
    "PySide6/Qt/plugins/imageformats/libqicns*",
    "PySide6/Qt/plugins/imageformats/libqico*",
    "PySide6/Qt/plugins/imageformats/libqmacheif*",
    "PySide6/Qt/plugins/imageformats/libqmacjp2*",
    "PySide6/Qt/plugins/imageformats/libqpdf*",
    "PySide6/Qt/plugins/imageformats/libqsvg*",
    "PySide6/Qt/plugins/imageformats/libqtga*",
    "PySide6/Qt/plugins/imageformats/libqwbmp*",
    "PySide6/Qt/plugins/networkinformation/*",
    "PySide6/Qt/plugins/platforminputcontexts/*",
    "PySide6/Qt/plugins/platforms/libqminimal*",
    "PySide6/Qt/plugins/platforms/libqoffscreen*",
    "PySide6/Qt/plugins/tls/*",
]

for root in ROOTS:
    if not root.exists():
        continue
    for rel in REMOVE_DIRS:
        path = root / rel
        if path.is_symlink() or path.is_file():
            path.unlink()
        elif path.exists():
            shutil.rmtree(path)

    for pattern in REMOVE_GLOBS:
        for path in root.glob(pattern):
            if path.is_symlink() or path.is_file():
                path.unlink()
            elif path.is_dir():
                shutil.rmtree(path)
