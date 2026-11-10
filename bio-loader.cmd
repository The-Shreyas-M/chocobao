@echo off
rem Wrapper: upstream bio-loader's pipx entry point is broken (hyphenated package dir), so run the script directly.
rem Uses the project-local Python environment (.venv) which has pyserial; ChocoBao.cmd creates it on first run.
"%~dp0.venv\Scripts\python.exe" "%~dp0repos\bio-loader\bio-loader\bio_loader.py" %*
