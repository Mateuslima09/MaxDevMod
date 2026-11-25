# Builds MaxDevMod (app + plugin) and installs it locally, with backups. Never pushes or releases.
# Usage (from anywhere):
#   powershell -ExecutionPolicy Bypass -File docs\fadiga-freio\instalar-local.ps1
#   ... -Prova "brake fade"      text the new DLL must contain (default "brake fade")
#   ... -SoPlugin                plugin-only change: build and swap only the DLL
#   ... -Restaurar               put back the last backup (exe, dll, ui)
param(
    [string]$Prova = "brake fade",
    [switch]$SoPlugin,
    [switch]$Restaurar
)
$ErrorActionPreference = "Stop"

$repo    = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$install = Join-Path $env:LOCALAPPDATA "MaxDevMod"
$backups = Join-Path $install "backups"
$stamp   = Get-Date -Format "yyyyMMdd-HHmmss"

function Find-GamePlugins {
    # Steam libraries from libraryfolders.vdf, then each game's plugins folder that already has westplay.dll.
    $vdf = "C:\Program Files (x86)\Steam\steamapps\libraryfolders.vdf"
    $libs = @("C:\Program Files (x86)\Steam")
    if (Test-Path $vdf) {
        $libs += Select-String -Path $vdf -Pattern '"path"\s+"([^"]+)"' | ForEach-Object { $_.Matches[0].Groups[1].Value -replace '\\\\', '\' }
    }
    $found = @()
    foreach ($lib in ($libs | Select-Object -Unique)) {
        foreach ($game in "American Truck Simulator", "Euro Truck Simulator 2") {
            $p = Join-Path $lib "steamapps\common\$game\bin\win_x64\plugins"
            if (Test-Path (Join-Path $p "westplay.dll")) { $found += $p }
        }
    }
    return $found
}

function Stop-App {
    Get-Process westplay -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 800
}

function Copy-Dll([string]$dll, [string]$dest) {
    # A loaded DLL can't be overwritten while the game runs, but it can be renamed aside.
    $target = Join-Path $dest "westplay.dll"
    if (Test-Path $target) {
        try { Copy-Item $dll $target -Force }
        catch {
            Rename-Item $target ("westplay.dll.old-" + $stamp)
            Copy-Item $dll $target
            Write-Host "  (game running: old DLL renamed aside, restart the game)"
        }
    } else { Copy-Item $dll $target }
}

$plugins = Find-GamePlugins
Write-Host "Install folder: $install"
Write-Host "Game plugin folders: $($plugins -join '; ')"

if ($Restaurar) {
    $last = Get-ChildItem $backups -Directory | Sort-Object Name -Descending | Select-Object -First 1
    if (-not $last) { throw "No backup in $backups" }
    Write-Host "Restoring $($last.FullName)"
    Stop-App
    if (Test-Path "$($last.FullName)\westplay.exe") { Copy-Item "$($last.FullName)\westplay.exe" $install -Force }
    if (Test-Path "$($last.FullName)\westplay.dll") {
        Copy-Item "$($last.FullName)\westplay.dll" $install -Force
        foreach ($p in $plugins) { Copy-Dll "$($last.FullName)\westplay.dll" $p }
    }
    if (Test-Path "$($last.FullName)\ui") {
        Remove-Item "$install\ui" -Recurse -Force -ErrorAction SilentlyContinue
        Copy-Item "$($last.FullName)\ui" "$install\ui" -Recurse
    }
    Start-Process "$install\westplay.exe"
    Write-Host "Restored. Restart the game if the plugin changed."
    exit 0
}

Push-Location $repo
try {
    # 1. Build. The app build copies an old target\release\westplay.dll over, so the plugin is built after it.
    if (-not $SoPlugin) {
        Write-Host "== npx tauri build"
        npx tauri build
        if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }
    }
    Write-Host "== cargo build --release -p westplay-plugin"
    cargo build --release -p westplay-plugin
    if ($LASTEXITCODE -ne 0) { throw "plugin build failed" }

    # The fresh DLL: the newest of the two target folders.
    $dll = @("$repo\wp-target\release\westplay.dll", "$repo\target\release\westplay.dll") + `
           @(if ($env:CARGO_TARGET_DIR) { "$env:CARGO_TARGET_DIR\release\westplay.dll" }) |
           Where-Object { Test-Path $_ } | Get-Item | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $dll) { throw "westplay.dll not found after build" }
    Copy-Item $dll.FullName "$repo\target\release\westplay.dll" -Force -ErrorAction SilentlyContinue

    # 2. Check the DLL really is the new one.
    $text = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($dll.FullName))
    if ($Prova -and $text.IndexOf($Prova) -lt 0) {
        throw "The DLL $($dll.FullName) has no '$Prova' in it: stale build. Touch a plugin file and run again."
    }
    Write-Host "DLL ok: $($dll.FullName) ($($dll.LastWriteTime))"

    $exe = @("$repo\target\release\westplay.exe", "$repo\wp-target\release\westplay.exe") |
           Where-Object { Test-Path $_ } | Get-Item | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $SoPlugin -and -not $exe) { throw "westplay.exe not found after build" }

    # 3. Back up what's installed.
    $bk = Join-Path $backups $stamp
    New-Item -ItemType Directory -Force $bk | Out-Null
    Stop-App
    foreach ($f in "westplay.exe", "westplay.dll") {
        if (Test-Path "$install\$f") { Copy-Item "$install\$f" $bk }
    }
    if (-not $SoPlugin -and (Test-Path "$install\ui")) { Copy-Item "$install\ui" "$bk\ui" -Recurse }
    Write-Host "Backup: $bk"

    # 4. Install.
    Copy-Item $dll.FullName "$install\westplay.dll" -Force
    foreach ($p in $plugins) { Copy-Dll $dll.FullName $p; Write-Host "  DLL -> $p" }
    if (-not $SoPlugin) {
        # The app may take a moment to let go of its exe after being stopped.
        for ($try = 1; ; $try++) {
            try { Copy-Item $exe.FullName "$install\westplay.exe" -Force -ErrorAction Stop; break }
            catch {
                if ($try -ge 15) { throw }
                Stop-App
                Start-Sleep -Seconds 1
            }
        }
        Remove-Item "$install\ui" -Recurse -Force -ErrorAction SilentlyContinue
        Copy-Item "$repo\ui\dist" "$install\ui" -Recurse
        Write-Host "  exe + ui -> $install"
    }

    # 5. Start the app again.
    Start-Process "$install\westplay.exe"
    Write-Host "Installed. Restart the game (the bridge changed). If Windows blocks the exe: -Restaurar"
}
finally { Pop-Location }
