$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$python = Join-Path $projectRoot ".venv\Scripts\python.exe"

if (-not (Test-Path -LiteralPath $python)) {
    throw "Missing promo-video virtual environment. Run: python -m venv promo-video\.venv; promo-video\.venv\Scripts\python.exe -m pip install pillow imageio-ffmpeg"
}

& $python (Join-Path $projectRoot "src\render_video.py")
if ($LASTEXITCODE -ne 0) {
    throw "Video render failed with exit code $LASTEXITCODE."
}
