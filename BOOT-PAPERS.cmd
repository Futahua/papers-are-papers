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

if exist "%PAPERS_EXE%" (
  echo Starting existing REAL build...
  start "" "%PAPERS_EXE%"
  exit /b 0
)

echo No REAL release build was found yet.
echo Building Papers from REAL now. This can take a minute.
echo.
pushd "%PAPERS_REAL%"
call npm.cmd run tauri -- build --no-bundle
set "BUILD_EXIT=%ERRORLEVEL%"
popd

if not "%BUILD_EXIT%"=="0" (
  echo.
  echo Build failed. Papers was not started.
  echo If this mentions missing packages, run npm install in REAL once, then try again.
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

echo Starting newly built REAL app...
start "" "%PAPERS_EXE%"
exit /b 0
