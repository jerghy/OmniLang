@echo off
cd /d "%~dp0"

set MSG=%1
if "%MSG%"=="" set MSG=Update

echo ========================================
echo   OmniLang - Push to Gitee + GitHub
echo ========================================
echo.

echo [1/2] git add .
git add .

echo.
echo [2/2] git commit -m "%MSG%"
git commit -m "%MSG%"

echo.
echo [3/3] git push origin
echo   -> Gitee  (HTTPS)
echo   -> GitHub (SSH)
git push origin

echo.
echo ========================================
echo   Done!
echo ========================================
pause
