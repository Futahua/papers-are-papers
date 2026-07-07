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
  powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -Command "$root=$env:PAPERS_REAL; $exe=$env:PAPERS_EXE; $exeTime=(Get-Item -LiteralPath $exe).LastWriteTimeUtc; $paths=@('src','src-tauri\src','src-tauri\tauri.conf.json','package.json','package-lock.json','vite.config.ts','tsconfig.json'); foreach ($relative in $paths) { $path=Join-Path $root $relative; if (Test-Path -LiteralPath $path -PathType Leaf) { if ((Get-Item -LiteralPath $path).LastWriteTimeUtc -gt $exeTime) { exit 1 } } elseif (Test-Path -LiteralPath $path -PathType Container) { if (Get-ChildItem -LiteralPath $path -Recurse -File | Where-Object { $_.LastWriteTimeUtc -gt $exeTime } | Select-Object -First 1) { exit 1 } } }; exit 0"
  if "%ERRORLEVEL%"=="0" (
    echo Starting existing REAL build...
    start "" "%PAPERS_EXE%"
    exit /b 0
  )
  echo Source files changed after the release exe was built.
  echo Rebuilding Papers from REAL now.
  echo.
) else (
  echo No REAL release build was found yet.
  echo Building Papers from REAL now. This can take a minute.
  echo.
)

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
start "" "%PAPERS_EXE%"
exit /b 0
