@echo off
rem ChocoBao one-click helper.
rem Plug the board in, press PROG (hold about 1 s), then double-click this file.
rem First run only: it builds a small Python environment inside this folder (needs internet once).
set "HERE=%~dp0"
set "PYW=%HERE%.venv\Scripts\pythonw.exe"
if not exist "%PYW%" (
  echo First run: setting up the helper's Python environment in "%HERE%.venv" ...
  py -3 -m venv "%HERE%.venv" 2>nul
  if not exist "%PYW%" python -m venv "%HERE%.venv"
  if not exist "%PYW%" (
    echo.
    echo Could not create the environment. Install Python 3 from python.org ^(tick "Add to PATH"^), then run this again.
    pause
    exit /b 1
  )
  "%HERE%.venv\Scripts\python.exe" -m pip install --quiet pyserial
  if errorlevel 1 (
    echo Could not install pyserial. Check your internet connection and run this again.
    pause
    exit /b 1
  )
)
start "" "%PYW%" "%HERE%tools\chocobao_gui.py"
