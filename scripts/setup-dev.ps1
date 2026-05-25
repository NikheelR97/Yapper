#Requires -Version 5.1
<#
.SYNOPSIS
    Yapper S0 Dev Environment Setup -- installs all tooling needed for development.

.DESCRIPTION
    Installs (via winget / direct download / cargo / npm):
      - Git, GitHub CLI
      - Node.js LTS
      - Visual Studio 2022 Build Tools (MSVC + Windows SDK) -- required by Rust on Windows
      - Rust (rustup), with cargo-watch, sqlx-cli, tauri-cli v2
      - Android Studio (SDK API 26+) + cmdline-tools
      - flyctl (Fly.io CLI)
      - wrangler (Cloudflare Workers CLI)
      - @capacitor/cli (Capacitor mobile)

    Account creation (Cloudflare, Neon, Firebase, Discord, Resend, etc.) is manual --
    a checklist is printed at the end.

.NOTES
    Run from an elevated PowerShell prompt (right-click -> "Run as Administrator").
    Safe to re-run -- each step checks whether the tool is already present.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# --- Helpers ------------------------------------------------------------------

function Write-Step  { param([string]$msg) Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$msg) Write-Host "    [OK]  $msg" -ForegroundColor Green }
function Write-Skip  { param([string]$msg) Write-Host "    [--]  $msg (already installed)" -ForegroundColor DarkGray }
function Write-Warn  { param([string]$msg) Write-Host "    [!!]  $msg" -ForegroundColor Yellow }
function Write-Fail  { param([string]$msg) Write-Host "    [XX]  $msg" -ForegroundColor Red }

function Test-Command { param([string]$name) return [bool](Get-Command $name -ErrorAction SilentlyContinue) }

function Install-Winget {
    param(
        [string]$Id,
        [string]$Name,
        [string]$CommandName
    )

    Write-Step "Installing $Name"

    # Prefer a direct command check first so we don't block on `winget list`
    # refreshing sources before we've even attempted the install.
    if ($CommandName -and (Test-Command $CommandName)) {
        Write-Skip $Name
        return
    }

    if (winget list --id $Id --exact -e --source winget 2>$null | Select-String $Id) {
        Write-Skip $Name
        return
    }

    winget install --id $Id --exact --source winget --silent --disable-interactivity `
        --accept-package-agreements --accept-source-agreements

    if ($LASTEXITCODE -ne 0) {
        throw "winget failed while installing $Name (exit code $LASTEXITCODE)."
    }

    Write-Ok $Name
}

function Install-Flyctl {
    Write-Step "Installing flyctl"

    if (Test-Command 'fly') {
        Write-Skip "flyctl"
        return
    }

    $wingetInstalled = $false
    try {
        winget install --id Superfly.flyctl --exact --source winget --silent --disable-interactivity `
            --accept-package-agreements --accept-source-agreements
        if ($LASTEXITCODE -eq 0) {
            $wingetInstalled = $true
        }
    } catch {
        $wingetInstalled = $false
    }

    if (-not $wingetInstalled) {
        Write-Warn "winget package for flyctl was not available; falling back to Fly.io's installer."
        $installScript = Join-Path $env:TEMP 'install-flyctl.ps1'
        Invoke-WebRequest -Uri 'https://fly.io/install.ps1' -OutFile $installScript -UseBasicParsing
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installScript
        Remove-Item $installScript -Force
    }

    $env:PATH = [System.Environment]::GetEnvironmentVariable('PATH','Machine') + ';' +
                [System.Environment]::GetEnvironmentVariable('PATH','User')

    if (Test-Command 'fly') {
        Write-Ok "flyctl"
        return
    }

    throw "flyctl installation completed, but 'fly' was not found on PATH. Open a new terminal and run 'fly version'."
}

# --- Admin check --------------------------------------------------------------

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Fail "This script must be run as Administrator. Right-click PowerShell -> 'Run as Administrator'."
    exit 1
}

# --- winget check -------------------------------------------------------------

Write-Step "Checking winget"
if (-not (Test-Command 'winget')) {
    Write-Fail "winget not found. Install 'App Installer' from the Microsoft Store, then re-run this script."
    exit 1
}
Write-Ok "winget $(winget --version)"

# --- 1. Git -------------------------------------------------------------------

Install-Winget -Id 'Git.Git' -Name 'Git' -CommandName 'git'

# Reload PATH so git is available immediately
$env:PATH = [System.Environment]::GetEnvironmentVariable('PATH','Machine') + ';' +
            [System.Environment]::GetEnvironmentVariable('PATH','User')

# --- 2. GitHub CLI ------------------------------------------------------------

Install-Winget -Id 'GitHub.cli' -Name 'GitHub CLI (gh)' -CommandName 'gh'

# --- 3. Node.js LTS -----------------------------------------------------------

Write-Step "Installing Node.js LTS"
if (Test-Command 'node') {
    $nodeVer = node --version
    Write-Skip "Node.js $nodeVer"
} else {
    winget install --id OpenJS.NodeJS.LTS --exact --silent --accept-package-agreements --accept-source-agreements
    $env:PATH = [System.Environment]::GetEnvironmentVariable('PATH','Machine') + ';' +
                [System.Environment]::GetEnvironmentVariable('PATH','User')
    Write-Ok "Node.js $(node --version)"
}

# --- 4. Visual Studio Build Tools (MSVC + Windows SDK) -----------------------
#     Required by Rust on Windows -- the linker lives here.

Write-Step "Installing Visual Studio 2022 Build Tools (C++ workload)"
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasVS = ($null -ne (Get-Command $vsWhere -ErrorAction SilentlyContinue)) -and
         (& $vsWhere -products * -requires Microsoft.VisualCpp.Tools.HostX64.TargetX64 2>$null)
if ($hasVS) {
    Write-Skip "MSVC C++ tools"
} else {
    winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --silent `
        --accept-package-agreements --accept-source-agreements `
        --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    Write-Ok "Visual Studio 2022 Build Tools"
    Write-Warn "A reboot may be required before Rust can use the MSVC linker."
}

# --- 5. Rust (rustup) ---------------------------------------------------------

Write-Step "Installing Rust via rustup"
if (Test-Command 'rustup') {
    $rustVer = rustc --version
    Write-Skip "Rust ($rustVer)"
    Write-Step "Updating Rust toolchain"
    rustup update stable
    Write-Ok "Rust toolchain up to date"
} else {
    $rustupUrl = 'https://win.rustup.rs/x86_64'
    $rustupExe = "$env:TEMP\rustup-init.exe"
    Write-Host "    Downloading rustup-init.exe..." -ForegroundColor DarkGray
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe -UseBasicParsing
    & $rustupExe -y --default-toolchain stable --profile minimal
    Remove-Item $rustupExe -Force

    # Add cargo/bin to current session PATH
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    Write-Ok "Rust $(rustc --version)"
}

# Ensure stable is the default
rustup default stable | Out-Null

# --- 6. Cargo tools -----------------------------------------------------------

Write-Step "Installing cargo-watch"
if (Test-Command 'cargo-watch') {
    Write-Skip "cargo-watch"
} else {
    cargo install cargo-watch
    Write-Ok "cargo-watch"
}

Write-Step "Installing sqlx-cli (postgres only, native-tls)"
if (Test-Command 'sqlx') {
    Write-Skip "sqlx-cli"
} else {
    cargo install sqlx-cli --no-default-features --features native-tls,postgres
    Write-Ok "sqlx-cli"
}

Write-Step "Installing Tauri CLI v2"
if (Test-Command 'cargo-tauri') {
    $tauriVer = cargo tauri --version 2>$null
    Write-Skip "tauri-cli ($tauriVer)"
} else {
    cargo install tauri-cli --version "^2" --locked
    Write-Ok "tauri-cli v2"
}

# --- 7. Android Studio --------------------------------------------------------

Install-Winget -Id 'Google.AndroidStudio' -Name 'Android Studio' -CommandName 'studio64'

Write-Warn "Android Studio installed. You still need to:"
Write-Warn "  1. Launch Android Studio -> SDK Manager -> SDK Platforms -> install API 26+ (Android 8.0)"
Write-Warn "  2. SDK Manager -> SDK Tools -> install NDK (Side by side) + CMake"
Write-Warn "  3. Set ANDROID_HOME: [System.Environment]::SetEnvironmentVariable('ANDROID_HOME', `"`$env:LOCALAPPDATA\Android\Sdk`", 'User')"
Write-Warn "  4. Add to PATH: `$ANDROID_HOME\platform-tools  and  `$ANDROID_HOME\cmdline-tools\latest\bin"

# --- 8. flyctl (Fly.io) -------------------------------------------------------

Install-Flyctl

# --- 9. npm global packages ---------------------------------------------------

Write-Step "Installing wrangler (Cloudflare Workers CLI)"
if (Test-Command 'wrangler') {
    Write-Skip "wrangler $(wrangler --version 2>$null)"
} else {
    npm install -g wrangler
    Write-Ok "wrangler"
}

Write-Step "Installing @capacitor/cli"
if (Test-Command 'cap') {
    Write-Skip "cap $(cap --version 2>$null)"
} else {
    npm install -g @capacitor/cli
    Write-Ok "@capacitor/cli"
}

# --- Summary ------------------------------------------------------------------

Write-Host ""
Write-Host "================================================================" -ForegroundColor Magenta
Write-Host "  TOOL INSTALLATION COMPLETE" -ForegroundColor Magenta
Write-Host "================================================================" -ForegroundColor Magenta
Write-Host ""

Write-Host "Installed tools -- verify versions:" -ForegroundColor White
$tools = @(
    @{ cmd='git';       arg='--version' },
    @{ cmd='gh';        arg='--version' },
    @{ cmd='node';      arg='--version' },
    @{ cmd='npm';       arg='--version' },
    @{ cmd='rustc';     arg='--version' },
    @{ cmd='cargo';     arg='--version' },
    @{ cmd='sqlx';      arg='--version' },
    @{ cmd='fly';       arg='version' },
    @{ cmd='wrangler';  arg='--version' },
    @{ cmd='cap';       arg='--version' }
)
foreach ($t in $tools) {
    if (Test-Command $t.cmd) {
        $ver = & $t.cmd $t.arg 2>$null | Select-Object -First 1
        Write-Host ("  {0,-12} {1}" -f $t.cmd, $ver) -ForegroundColor Green
    } else {
        Write-Host ("  {0,-12} NOT FOUND (may need a new shell)" -f $t.cmd) -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "================================================================" -ForegroundColor Yellow
Write-Host "  MANUAL STEPS (accounts + credentials -- cannot be scripted)" -ForegroundColor Yellow
Write-Host "================================================================" -ForegroundColor Yellow
Write-Host ""
Write-Host "  [ ] 1.  Android Studio -- launch and install SDK API 26+ + NDK (see warning above)" -ForegroundColor White
Write-Host "  [ ] 2.  Apple FamilyControls entitlement -- apply NOW (1-4 week lead time):" -ForegroundColor White
Write-Host "           https://developer.apple.com/contact/request/family-controls-distribution" -ForegroundColor DarkGray
Write-Host "  [ ] 3.  Cloudflare -- create R2 bucket, D1 database, KV namespace, Pages project" -ForegroundColor White
Write-Host "           https://dash.cloudflare.com" -ForegroundColor DarkGray
Write-Host "  [ ] 4.  Fly.io -- create account + run:  fly auth login" -ForegroundColor White
Write-Host "           https://fly.io" -ForegroundColor DarkGray
Write-Host "  [ ] 5.  Neon -- create project, copy DATABASE_URL to backend/.env" -ForegroundColor White
Write-Host "           https://console.neon.tech" -ForegroundColor DarkGray
Write-Host "  [ ] 6.  Firebase -- create project, enable FCM, download service-account.json" -ForegroundColor White
Write-Host "           https://console.firebase.google.com" -ForegroundColor DarkGray
Write-Host "  [ ] 7.  Discord Developer App -- create app, copy Client ID + Secret" -ForegroundColor White
Write-Host "           https://discord.com/developers/applications" -ForegroundColor DarkGray
Write-Host "  [ ] 8.  Google Cloud OAuth2 -- create credentials (Web application)" -ForegroundColor White
Write-Host "           https://console.cloud.google.com/apis/credentials" -ForegroundColor DarkGray
Write-Host "  [ ] 9.  Resend -- create account, get API key" -ForegroundColor White
Write-Host "           https://resend.com" -ForegroundColor DarkGray
Write-Host "  [ ] 10. Apple Developer Program (`$99/year) -- only needed at App Store submission" -ForegroundColor White
Write-Host "           https://developer.apple.com/programs" -ForegroundColor DarkGray
Write-Host "  [ ] 11. Google Play Console (`$25 one-time) -- only needed at Play Store submission" -ForegroundColor White
Write-Host "           https://play.google.com/console" -ForegroundColor DarkGray
Write-Host "  [ ] 12. Purchase domain on Porkbun (yapperhq.com or alternative)" -ForegroundColor White
Write-Host "           https://porkbun.com" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Once accounts are ready, copy credentials to:" -ForegroundColor White
Write-Host "    backend/.env   (DATABASE_URL, JWT_PRIVATE_KEY, JWT_PUBLIC_KEY, RESEND_API_KEY, ...)" -ForegroundColor DarkGray
Write-Host "    .github/       (FLY_API_TOKEN secret in repo settings)" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Open a NEW terminal after this script so PATH changes take effect." -ForegroundColor Cyan
Write-Host ""
