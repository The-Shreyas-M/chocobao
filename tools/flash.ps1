# Flash the Dabao with Xous images (UF2 copy method).
#   .\tools\flash.ps1                 copy loader+xous+apps (first time, or after kernel/loader change)
#   .\tools\flash.ps1 -Mode apps      copy apps.uf2 only (normal iteration)
#   .\tools\flash.ps1 -Build          build first (cargo xtask dabao dabao-console), then flash
# Needs the board in BOOTLOADER mode (BAOCHIP drive visible). Refuses anything else (e.g. ALTCHIP = boot1 update flow).
param(
    [ValidateSet('all', 'apps')][string]$Mode = 'all',
    [switch]$Build,
    [string]$Xous = (Join-Path $PSScriptRoot '..\repos\xous-core')
)
$ErrorActionPreference = 'Stop'
$env:Path += ";$env:USERPROFILE\.cargo\bin"
$release = Join-Path $Xous 'target\riscv32imac-unknown-xous-elf\release'
$files = if ($Mode -eq 'apps') { @('apps.uf2') } else { @('loader.uf2', 'xous.uf2', 'apps.uf2') }
$py = "$env:LOCALAPPDATA\pipx\pipx\venvs\bio-loader\Scripts\python.exe"
$baoport = Join-Path $PSScriptRoot 'baoport.py'

if ($Build) {
    Write-Host '== Building' -ForegroundColor Cyan
    Push-Location $Xous
    try { cargo xtask dabao dabao-console; if ($LASTEXITCODE) { throw "build failed ($LASTEXITCODE)" } } finally { Pop-Location }
}
foreach ($f in $files) { if (-not (Test-Path (Join-Path $release $f))) { throw "missing $f in $release - build first (-Build)" } }

Write-Host '== Waiting for BAOCHIP drive (up to 60 s)' -ForegroundColor Cyan
$vol = $null
for ($i = 0; $i -lt 60 -and -not $vol; $i++) {
    $all = Get-Volume | Where-Object FileSystemLabel -match '^(BAOCHIP|ALTCHIP)$'
    if ($all | Where-Object FileSystemLabel -eq 'ALTCHIP') { throw 'ALTCHIP drive present = boot1 update flow. Stopping on purpose.' }
    $vol = $all | Where-Object FileSystemLabel -eq 'BAOCHIP' | Select-Object -First 1
    if (-not $vol) { Start-Sleep 1 }
}
if (-not $vol) { throw 'No BAOCHIP drive. Replug USB (hold PROG while plugging in if it does not appear).' }
$dest = "$($vol.DriveLetter):\"
Write-Host "Target $dest"

foreach ($f in $files) {
    Copy-Item (Join-Path $release $f) $dest -ErrorAction Stop
    Write-Host "copied $f"
    Start-Sleep 2
}
Write-Host '== Now press and HOLD PROG until the drive disappears (board reboots).' -ForegroundColor Yellow

Write-Host '== Waiting for the app to enumerate (up to 90 s)' -ForegroundColor Cyan
for ($i = 0; $i -lt 90; $i++) {
    $out = & $py $baoport 2>$null
    if ($out -match 'RUNNING APP') { Write-Host "OK: $out" -ForegroundColor Green; exit 0 }
    Start-Sleep 1
}
Write-Host 'App did not appear. Check: did the drive unmount? PID 6196 still = still in bootloader.' -ForegroundColor Red
exit 1
