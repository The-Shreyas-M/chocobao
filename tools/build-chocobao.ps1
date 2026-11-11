# Build the ChocoBao app.
# Our source of truth is  <project>\chocobao\  ; this script mirrors it into xous-core's workspace
# (apps-dabao\chocobao), makes sure the workspace lists it, then runs `cargo xtask dabao chocobao`.
# Output: repos\xous-core\target\riscv32imac-unknown-xous-elf\release\{loader,xous,apps}.uf2
# Flash with:  python tools\baoflash.py <apps.uf2>   (board in bootloader mode)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$xous = Join-Path $root 'repos\xous-core'
$src  = Join-Path $root 'chocobao'
$dst  = Join-Path $xous 'apps-dabao\chocobao'
$env:Path += ";$env:USERPROFILE\.cargo\bin"

robocopy $src $dst /MIR /XD target /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy failed ($LASTEXITCODE)" }
$global:LASTEXITCODE = 0

$cargo = Join-Path $xous 'Cargo.toml'
$text = [IO.File]::ReadAllText($cargo)
if ($text -notmatch 'apps-dabao/chocobao') {
    $text = $text -replace '(\r?\n)(\s*)"apps-dabao/dabao-console",', "`$1`$2`"apps-dabao/dabao-console`",`$1`$2`"apps-dabao/chocobao`","
    [IO.File]::WriteAllText($cargo, $text)
    Write-Host 'added apps-dabao/chocobao to the xous-core workspace'
}

Push-Location $xous
try { cargo xtask dabao chocobao; if ($LASTEXITCODE) { throw "build failed ($LASTEXITCODE)" } } finally { Pop-Location }

$rel = Join-Path $xous 'target\riscv32imac-unknown-xous-elf\release'
# the last 4 KiB page of the app region (0x3D9000) stores the settings: the image must stay below it
$img = (Get-Item (Join-Path $rel 'apps.img')).Length
if ($img -gt 0xD8000) { throw ('apps.img is {0} bytes, over the {1} limit: it would overwrite the settings page' -f $img, 0xD8000) }
Write-Host ('apps.img {0:N0} bytes, {1:N0} free before the settings page' -f $img, (0xD8000 - $img))
foreach ($f in 'loader.uf2', 'xous.uf2', 'apps.uf2') { $i = Get-Item (Join-Path $rel $f); '{0,-11} {1,8:N0} KiB  {2}' -f $f, ($i.Length / 1KB), $i.LastWriteTime }
