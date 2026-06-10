@echo off
cd /d "%~dp0"

set MSG=%1
if "%MSG%"=="" set MSG=Update

git add .
git commit -m "%MSG%"
git push
pause
