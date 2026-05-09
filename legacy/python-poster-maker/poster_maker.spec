# PyInstaller spec for Poster Maker (Tk GUI, much smaller than Qt)

block_cipher = None

a = Analysis(
    ["packaging/poster_maker_gui.py"],
    pathex=["src"],
    binaries=[],
    datas=[],
    hiddenimports=[],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["PySide6", "shiboken6", "PIL", "Pillow", "numpy", "matplotlib", "pandas", "scipy"],
    noarchive=False,
)
pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="PosterMaker",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name="PosterMaker",
)
app = BUNDLE(
    coll,
    name="PosterMaker.app",
    icon=None,
    bundle_identifier="tw.mics.postermaker",
    info_plist={
        "CFBundleDisplayName": "Poster Maker",
        "CFBundleName": "Poster Maker",
        "CFBundleShortVersionString": "0.1.1",
        "CFBundleVersion": "0.1.1",
        "NSHighResolutionCapable": True,
    },
)
