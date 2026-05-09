$ErrorActionPreference = "Stop"
$Version = if ($env:VERSION) { $env:VERSION } else { "0.1.0" }

python -m pip install --upgrade pip
python -m pip install -e . pyinstaller
pyinstaller --noconfirm --clean poster_maker.spec
python scripts/prune_pyinstaller_bundle.py

New-Item -ItemType Directory -Force -Path release | Out-Null
Compress-Archive -Path dist\PosterMaker\* -DestinationPath "release\PosterMaker-$Version-windows.zip" -Force

if (Get-Command iscc -ErrorAction SilentlyContinue) {
  iscc /DMyAppVersion=$Version installer\PosterMaker.iss
}
