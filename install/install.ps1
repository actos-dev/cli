<#
install.ps1 - Install the Actos CLI (`actos`) on Windows.

Install Actos by building it from source with Cargo (recommended). PowerShell
runs it directly without any additional steps:

    powershell -ExecutionPolicy Bypass -File install.ps1
    # or
    pwsh -ExecutionPolicy Bypass -File install.ps1

Or run it in-memory:

    & ([scriptblock]::Create((Invoke-RestMethod https://actos.com.tr/cli/install.ps1)))

This script works under BOTH Windows PowerShell 5.1 and PowerShell 7 (pwsh).

Parameters:
  -Version <x.y.z>        Install a specific published version (default: latest).
  -NoModifyPath           Do not print PATH / env-var advice.
  -Force                  Reinstall over an existing `actos` binary.
  -Help                   Show this help and exit.

Exit codes:
  0  success
  2  missing required tooling (Cargo/Rust) - nothing installed
  3  `cargo install` failed
#>

[CmdletBinding()]
param(
    [string]$Version = "",
    [switch]$NoModifyPath,
    [switch]$Force,
    [switch]$Help
)

# ---------------------------------------------------------------------------
# Small logger helpers. Functions are defined BEFORE their call sites so the
# script behaves identically on Windows PowerShell 5.1 and PowerShell 7.
# ---------------------------------------------------------------------------
function Write-Info { Write-Host "==> $args" -ForegroundColor Cyan }
function Write-Ok   { Write-Host "ok  $args" -ForegroundColor Green }
function Write-Warn { Write-Host "warn $args" -ForegroundColor Yellow }
function Write-Err  { Write-Host "err  $args" -ForegroundColor Red }

function fail {
    param([string]$Message, [int]$Code = 1)
    Write-Err $Message
    exit $Code
}

function Show-Help {
    @'

Actos CLI installer (Windows / PowerShell)

Usage:
    powershell -ExecutionPolicy Bypass -File install.ps1 [options]
    pwsh -ExecutionPolicy Bypass -File install.ps1            [options]

Options:
  -Version <x.y.z>   Install a specific published version (default: latest).
  -NoModifyPath     Do not print PATH / environment-variable advice.
  -Force            Reinstall even if `actos` is already installed.
  -Help             Show this help and exit.

Examples:
    powershell -ExecutionPolicy Bypass -File install.ps1
    powershell -ExecutionPolicy Bypass -File install.ps1 -Version 0.1.0
    powershell -ExecutionPolicy Bypass -File install.ps1 -NoModifyPath

For a healthy default setup, make sure Rust is installed first:
    winget install Rustlang.Rustup
'@
    exit 0
}

# ---------------------------------------------------------------------------
# Help / parse-time switches.
# ---------------------------------------------------------------------------
if ($Help) { Show-Help }

Write-Info "Actos CLI installer"
Write-Info ""

# ---------------------------------------------------------------------------
# Locate Cargo. We build from source, so Rust/Cargo are required.
# ---------------------------------------------------------------------------
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    Write-Err ""
    Write-Err "Cargo (Rust) was not found on this system."
    Write-Err "Actos is installed from source, so Cargo is required."
    Write-Err ""
    Write-Err "Install Rust with rustup (recommended):"
    Write-Err "    winget install Rustlang.Rustup"
    Write-Err "    # or download from https://rustup.rs"
    Write-Err ""
    Write-Err "After installing, open a NEW terminal (so your PATH refreshes)"
    Write-Err "and re-run this installer."
    fail "Cargo is required to install Actos." 2
}

Write-Info "Found Cargo at: $($cargo.Source)"

$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { "$env:USERPROFILE\.cargo" }
$cargoBin  = if ($env:CARGO_BIN)  { $env:CARGO_BIN }  else { "$cargoHome\bin" }

$actosExe = Join-Path $cargoBin "actos.exe"
Write-Info "Cargo bin dir: $cargoBin"
Write-Ok ""

# ---------------------------------------------------------------------------
# Already installed? Skip the (potentially slow) rebuild unless -Force.
# ---------------------------------------------------------------------------
if ((Test-Path $actosExe) -and (-not $Force)) {
    Write-Warn "'actos' is already installed at: $actosExe"
    Write-Warn "Re-run with -Force to reinstall, or -Version to pick a version."
} else {
    Write-Info "Installing the Actos CLI from crates.io ..."
    Write-Info "  (cargo install actos-cli --locked)"
    $versionArg = ""
    if ($Version) {
        $versionArg = "--version $Version"
        Write-Info "Running: cargo install actos-cli --locked --version $Version"
    } else {
        Write-Info "Running: cargo install actos-cli --locked"
    }

    $argv = @("install", "actos-cli", "--locked")
    if ($Version) { $argv += @("--version", $Version) }
    if ($Force)   { $argv += "--force" }

    & cargo @argv
    if ($LASTEXITCODE -ne 0) {
        fail "cargo install failed (exit code $LASTEXITCODE)." 3
    }
    Write-Ok ""
}

# ---------------------------------------------------------------------------
# Verify the binary actually runs.
# ---------------------------------------------------------------------------
if (-not (Test-Path $actosExe)) {
    # Fall back to PATH resolution in case the default bin dir was wrong.
    $resolved = Get-Command actos -ErrorAction SilentlyContinue
    if ($resolved) { $actosExe = $resolved.Source }
    else { fail "Installer finished but 'actos' was not found in $cargoBin." 3 }
}

$installedVersion = (& $actosExe --version 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $installedVersion) {
    fail "Installed binary does not run. Please check the Cargo output above." 3
}

Write-Ok "Actos installed successfully: $($installedVersion -join ' ') ($actosExe)"

# ---------------------------------------------------------------------------
# PATH check + environment-variable advice.
# ---------------------------------------------------------------------------
if (-not $NoModifyPath) {
    $pathEntries = $env:PATH -split ';'
    $matched = $pathEntries | Where-Object { $_ -and $_.TrimEnd('\').ToLower() -eq $cargoBin.TrimEnd('\').ToLower() }

    Write-Info ""
    if ($matched) {
        Write-Ok "~\.cargo\bin is already on your PATH. You're all set."
    } else {
        Write-Warn "~\.cargo\bin is NOT on your current PATH."
        Write-Warn "Add it so you can run 'actos' from anywhere. In PowerShell:"
        Write-Info ""
        Write-Info "  [Environment]::SetEnvironmentVariable('Path',"
        Write-Info "      [Environment]::GetEnvironmentVariable('Path','User') + ';$cargoBin',"
        Write-Info "      'User')"
        Write-Info ""
        Write-Info "  # then open a NEW terminal window."
        Write-Info ""
        Write-Warn "Until then, run Actos with its full path:  $actosExe --help"
    }
}

# ---------------------------------------------------------------------------
# Final summary + next steps.
# ---------------------------------------------------------------------------
Write-Info ""
Write-Ok "Actos is ready. Try it:"
Write-Info ""
Write-Info "    actos --help"
Write-Info ""
Write-Info "Uninstall later with:"
Write-Info "    cargo uninstall actos-cli"
Write-Info ""
exit 0