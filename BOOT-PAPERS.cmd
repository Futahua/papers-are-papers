@echo off
setlocal

set "PAPERS_REAL=C:\This is Minh\LapSlop brotherhood\Programs\Papers are papers\REAL"
set "PAPERS_EXE=%PAPERS_REAL%\src-tauri\target\release\papers.exe"

title Papers Launcher
echo Papers launcher
echo.
echo Source:
echo   %PAPERS_REAL%
echo.

if not exist "%PAPERS_REAL%\package.json" (
  echo Could not find the Papers REAL project folder.
  echo Expected:
  echo   %PAPERS_REAL%
  echo.
  pause
  exit /b 1
)

echo Closing any running Papers app process...
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -Command "Get-Process -Name papers -ErrorAction SilentlyContinue | ForEach-Object { $pidToStop=$_.Id; try { Stop-Process -Id $pidToStop -Force -ErrorAction Stop } catch { Write-Host ('Could not stop papers.exe PID ' + $pidToStop + ': ' + $_.Exception.Message) } }"
echo.

echo Rebuilding Papers from REAL now. This can take a minute.
echo.

pushd "%PAPERS_REAL%"
call npm.cmd run tauri -- build --no-bundle
set "BUILD_EXIT=%ERRORLEVEL%"
popd

if not "%BUILD_EXIT%"=="0" (
  echo.
  echo Build failed. Papers was not started.
  echo If this mentions the exe is locked, close Papers completely or end papers.exe in Task Manager, then try again.
  echo.
  pause
  exit /b %BUILD_EXIT%
)

if not exist "%PAPERS_EXE%" (
  echo.
  echo Build finished, but the app exe was not found at:
  echo   %PAPERS_EXE%
  echo.
  pause
  exit /b 1
)

echo Starting rebuilt REAL app...
set "PAPERS_REPO_PATH=%PAPERS_REAL%"
start "" "%PAPERS_EXE%"
exit /b 0
