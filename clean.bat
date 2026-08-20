@echo off
setlocal EnableExtensions

rem Removes only reproducible build output and development caches.
rem It does not remove source files, configuration, node_modules, or local app data.

pushd "%~dp0" >nul || (
  echo Unable to open the project directory.
  exit /b 1
)

echo Cleaning ReportManager build output and temporary files...

call :remove_dir "dist"
call :remove_dir "dist-ssr"
call :remove_dir "coverage"
call :remove_dir ".vite"
call :remove_dir "node_modules\.vite"
call :remove_dir "node_modules\.cache"
call :remove_dir "src-tauri\target"
call :remove_dir "src-tauri\gen\schemas"

call :remove_file "*.tsbuildinfo"
call :remove_file "*.log"
call :remove_file "npm-debug.log*"
call :remove_file "yarn-debug.log*"
call :remove_file "yarn-error.log*"
call :remove_file "pnpm-debug.log*"

popd
echo Cleanup complete.
exit /b 0

:remove_dir
if exist "%~1\" (
  echo   Removing directory: %~1
  rmdir /s /q "%~1"
)
exit /b 0

:remove_file
if exist "%~1" (
  echo   Removing files: %~1
  del /q "%~1"
)
exit /b 0
