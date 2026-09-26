<#
.SYNOPSIS
    Installs cmdpal for the current user (no admin rights needed).
.EXAMPLE
    irm https://github.com/SurajaHasarinda/cmdpal/releases/latest/download/install.ps1 | iex
.EXAMPLE
    .\install.ps1 -Version 0.2.0
.EXAMPLE
    .\install.ps1 -Uninstall -Purge
#>
[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\cmdpal"),
    # Release download URL, or a local directory holding the release assets (offline installs, CI).
    [string]$BaseUrl,
    [switch]$NoPathUpdate,
    [switch]$Uninstall,
    # With -Uninstall: also delete the usage history in %APPDATA%\cmdpal.
    [switch]$Purge
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue" # Windows PowerShell 5.1 downloads are ~10x slower with the progress bar

$Repo = if ($env:CMDPAL_REPO) { $env:CMDPAL_REPO } else { "SurajaHasarinda/cmdpal" }
$Archive = "cmdpal-x86_64-pc-windows-msvc.zip"

function Write-Step($message) { Write-Host "==> $message" }

function Get-UserPath {
    [Environment]::GetEnvironmentVariable("Path", "User")
}

function Test-SamePath($a, $b) {
    $a.TrimEnd("\") -ieq $b.TrimEnd("\")
}

function Remove-FromUserPath($dir) {
    $kept = @((Get-UserPath) -split ";" | Where-Object { $_ -and -not (Test-SamePath $_ $dir) })
    [Environment]::SetEnvironmentVariable("Path", ($kept -join ";"), "User")
}

function Add-ToUserPath($dir) {
    $entries = @((Get-UserPath) -split ";" | Where-Object { $_ })
    if ($entries | Where-Object { Test-SamePath $_ $dir }) { return $false }
    # SetEnvironmentVariable at User scope also broadcasts WM_SETTINGCHANGE to running shells.
    [Environment]::SetEnvironmentVariable("Path", (($entries + $dir) -join ";"), "User")
    return $true
}

function Invoke-Uninstall {
    if (Test-Path $InstallDir) {
        try { Remove-Item -Recurse -Force $InstallDir }
        catch { throw "Could not remove $InstallDir. Close any open cmdpal window and re-run." }
        Write-Step "Removed $InstallDir"
    }
    Remove-FromUserPath $InstallDir
    if ($Purge) {
        $data = Join-Path $env:APPDATA "cmdpal"
        if (Test-Path $data) { Remove-Item -Recurse -Force $data; Write-Step "Removed history in $data" }
    }
    Write-Host "cmdpal uninstalled."
}

function Get-Asset($base, $name, $destination) {
    if (Test-Path -LiteralPath $base -PathType Container) {
        Copy-Item -LiteralPath (Join-Path $base $name) -Destination $destination
        return
    }
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$name" -OutFile $destination
}

function Assert-Checksum($archivePath, $sumsPath) {
    $line = Get-Content $sumsPath | Where-Object { $_ -match "^\s*([0-9a-fA-F]{64})\s+\*?$([regex]::Escape($Archive))\s*$" } | Select-Object -First 1
    if (-not $line) { throw "SHA256SUMS has no entry for $Archive; refusing to install." }
    $expected = ($line -split "\s+" | Where-Object { $_ })[0]
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash
    if ($actual -ine $expected) {
        throw "Checksum mismatch for $Archive (expected $expected, got $actual). The download is corrupt or has been tampered with; nothing was installed."
    }
}

function Invoke-Install {
    $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    if ($arch -ne "AMD64") {
        throw "Only 64-bit Intel/AMD Windows has prebuilt binaries (found $arch). Build from source: cargo install --git https://github.com/$Repo"
    }
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    if (-not $BaseUrl) {
        $BaseUrl = if ($Version -eq "latest") { "https://github.com/$Repo/releases/latest/download" }
                   else { "https://github.com/$Repo/releases/download/v$($Version.TrimStart('v'))" }
    }

    $work = Join-Path ([IO.Path]::GetTempPath()) ("cmdpal-install-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        Write-Step "Downloading $Archive ($Version)"
        Get-Asset $BaseUrl $Archive (Join-Path $work $Archive)
        Get-Asset $BaseUrl "SHA256SUMS" (Join-Path $work "SHA256SUMS")

        Write-Step "Verifying SHA-256"
        Assert-Checksum (Join-Path $work $Archive) (Join-Path $work "SHA256SUMS")

        $stage = Join-Path $work "stage"
        Expand-Archive -LiteralPath (Join-Path $work $Archive) -DestinationPath $stage -Force

        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        $target = Join-Path $InstallDir "cmdpal.exe"
        $pending = "$target.new"
        Copy-Item -LiteralPath (Join-Path $stage "cmdpal.exe") -Destination $pending -Force
        try { Move-Item -LiteralPath $pending -Destination $target -Force }
        catch {
            Remove-Item -LiteralPath $pending -Force -ErrorAction SilentlyContinue
            throw "cmdpal.exe is in use. Close any open cmdpal window and re-run the installer."
        }
        Write-Step "Installed to $InstallDir"

        if (-not $NoPathUpdate) {
            if (Add-ToUserPath $InstallDir) { Write-Step "Added $InstallDir to your user PATH" }
            if (-not ($env:Path -split ";" | Where-Object { $_ -and (Test-SamePath $_ $InstallDir) })) {
                $env:Path = "$env:Path;$InstallDir"
            }
        }
        Write-Host ("Installed " + (& $target --version))
        if (-not $NoPathUpdate) { Write-Host "Open a new terminal, then try:  cmdpal delete directory" }
    }
    finally {
        Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
    }
}

if ($Uninstall) { Invoke-Uninstall } else { Invoke-Install }
