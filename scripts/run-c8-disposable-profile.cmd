@echo off
setlocal

cd /d "%~dp0"

echo Starting the SiteDatum C8 disposable-profile test...
echo.

pwsh.exe -NoProfile -File "%~dp0c8-disposable-profile.ps1" -InstallerPath "%~dp0SiteDatum_1.4.0_x64-setup.exe" -Execute -DisposableProfileAcknowledged
set "siteDatumExitCode=%ERRORLEVEL%"

echo.
if not "%siteDatumExitCode%"=="0" (
  echo The C8 runner stopped with exit code %siteDatumExitCode%.
  echo Leave this window open and send Codex a photo of the message above.
) else (
  echo The C8 runner completed successfully.
)
echo.
pause
exit /b %siteDatumExitCode%
