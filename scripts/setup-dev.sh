#!/usr/bin/env bash
#
# Yapper S0 Dev Environment Setup -- CachyOS / Arch Linux
#
# Installs (via pacman / paru / rustup / cargo / npm):
#   - Git, GitHub CLI
#   - Node.js LTS + npm
#   - base-devel (gcc, make, pkgconf — required by Rust + native modules)
#   - Tauri v2 system dependencies (webkit2gtk-4.1, gtk3, etc.)
#   - Rust (rustup), with cargo-watch, sqlx-cli, tauri-cli v2
#   - Android Studio (AUR) -- SDK install is still manual
#   - flyctl (Fly.io CLI, AUR)
#   - wrangler (Cloudflare Workers CLI)
#   - @capacitor/cli (Capacitor mobile)
#
# Account creation (Cloudflare, Neon, Firebase, Discord, Resend, etc.) is manual --
# a checklist is printed at the end.
#
# Usage:   ./scripts/setup-dev.sh
# Notes:   Do NOT run as root. The script will sudo where needed.
#          Safe to re-run -- each step is idempotent.

set -euo pipefail

# --- Colours ------------------------------------------------------------------

if [[ -t 1 ]]; then
    C_CYAN=$'\033[36m'
    C_GREEN=$'\033[32m'
    C_GREY=$'\033[90m'
    C_YELLOW=$'\033[33m'
    C_RED=$'\033[31m'
    C_MAGENTA=$'\033[35m'
    C_WHITE=$'\033[37m'
    C_RESET=$'\033[0m'
else
    C_CYAN= C_GREEN= C_GREY= C_YELLOW= C_RED= C_MAGENTA= C_WHITE= C_RESET=
fi

write_step() { printf '\n%s==> %s%s\n' "$C_CYAN" "$1" "$C_RESET"; }
write_ok()   { printf '    %s[OK]%s  %s\n' "$C_GREEN" "$C_RESET" "$1"; }
write_skip() { printf '    %s[--]%s  %s (already installed)\n' "$C_GREY" "$C_RESET" "$1"; }
write_warn() { printf '    %s[!!]%s  %s\n' "$C_YELLOW" "$C_RESET" "$1"; }
write_fail() { printf '    %s[XX]%s  %s\n' "$C_RED" "$C_RESET" "$1"; }

have() { command -v "$1" >/dev/null 2>&1; }
pkg_installed() { pacman -Q "$1" >/dev/null 2>&1; }

# --- Sanity checks ------------------------------------------------------------

if [[ $EUID -eq 0 ]]; then
    write_fail "Do not run this script as root. It will sudo where required."
    exit 1
fi

if ! have pacman; then
    write_fail "pacman not found. This script targets CachyOS / Arch-based distros only."
    exit 1
fi

write_step "Refreshing sudo credential cache"
sudo -v
# Keep sudo alive for the duration of the script.
( while true; do sudo -nv; sleep 50; done ) &
SUDO_KEEPALIVE_PID=$!
trap 'kill "$SUDO_KEEPALIVE_PID" 2>/dev/null || true' EXIT

# --- AUR helper ---------------------------------------------------------------

write_step "Detecting AUR helper"
if have paru; then
    AUR=paru
    write_ok "paru $(paru --version | head -n1)"
elif have yay; then
    AUR=yay
    write_ok "yay $(yay --version | head -n1)"
else
    write_warn "No AUR helper found (paru/yay). Installing paru via makepkg..."
    sudo pacman -S --needed --noconfirm base-devel git
    tmp=$(mktemp -d)
    git clone https://aur.archlinux.org/paru-bin.git "$tmp/paru-bin"
    ( cd "$tmp/paru-bin" && makepkg -si --noconfirm )
    rm -rf "$tmp"
    AUR=paru
    write_ok "paru installed"
fi

# --- Pacman packages ----------------------------------------------------------
#
# Bundled into a single pacman call so the resolver runs once and the user
# only confirms once. --needed skips anything already installed.

write_step "Installing pacman packages (git, gh, nodejs LTS, base-devel, Tauri v2 deps)"
PACMAN_PKGS=(
    git
    github-cli
    nodejs-lts-jod        # Node 22 LTS — matches Yapper CI matrix
    npm
    base-devel
    curl
    wget
    file
    openssl
    pkgconf
    # --- Tauri v2 Linux runtime deps ---
    webkit2gtk-4.1
    gtk3
    libappindicator-gtk3
    librsvg
    # --- nice-to-have for native builds ---
    cmake
    clang
)
sudo pacman -S --needed --noconfirm "${PACMAN_PKGS[@]}"
write_ok "pacman packages"

# --- Rust (rustup) ------------------------------------------------------------

write_step "Installing Rust via rustup"
if have rustup; then
    write_skip "rustup ($(rustc --version 2>/dev/null || echo 'no toolchain yet'))"
    write_step "Updating Rust stable toolchain"
    rustup update stable
    write_ok "Rust toolchain up to date"
else
    # Use the official Arch package — it's the rustup binary, not the system rust.
    sudo pacman -S --needed --noconfirm rustup
    rustup default stable
    # shellcheck disable=SC1090
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
    write_ok "Rust $(rustc --version)"
fi

rustup default stable >/dev/null

# Make cargo available in this shell even if .cargo/env was not sourced yet.
export PATH="$HOME/.cargo/bin:$PATH"

# --- Cargo tools --------------------------------------------------------------

write_step "Installing cargo-watch"
if have cargo-watch; then
    write_skip "cargo-watch"
else
    cargo install cargo-watch --locked
    write_ok "cargo-watch"
fi

write_step "Installing sqlx-cli (postgres only, native-tls)"
if have sqlx; then
    write_skip "sqlx-cli ($(sqlx --version 2>/dev/null))"
else
    cargo install sqlx-cli --no-default-features --features native-tls,postgres --locked
    write_ok "sqlx-cli"
fi

write_step "Installing Tauri CLI v2"
if have cargo-tauri; then
    write_skip "tauri-cli ($(cargo tauri --version 2>/dev/null))"
else
    cargo install tauri-cli --version "^2" --locked
    write_ok "tauri-cli v2"
fi

# --- Android Studio (AUR) -----------------------------------------------------

write_step "Installing Android Studio (AUR — large download, may take a while)"
if pkg_installed android-studio; then
    write_skip "android-studio"
else
    "$AUR" -S --needed --noconfirm android-studio
    write_ok "android-studio"
fi

write_warn "Android Studio installed. You still need to:"
write_warn "  1. Launch Android Studio -> SDK Manager -> SDK Platforms -> install API 26+ (Android 8.0)"
write_warn "  2. SDK Manager -> SDK Tools -> install NDK (Side by side) + CMake"
write_warn "  3. Add to your shell profile (~/.config/fish/config.fish or ~/.bashrc):"
write_warn "       set -x ANDROID_HOME \$HOME/Android/Sdk        # fish"
write_warn "       export ANDROID_HOME=\"\$HOME/Android/Sdk\"      # bash/zsh"
write_warn "  4. Add to PATH: \$ANDROID_HOME/platform-tools and \$ANDROID_HOME/cmdline-tools/latest/bin"

# --- flyctl (AUR) -------------------------------------------------------------

write_step "Installing flyctl (Fly.io CLI)"
if have flyctl || have fly; then
    write_skip "flyctl"
else
    "$AUR" -S --needed --noconfirm flyctl-bin
    write_ok "flyctl"
fi

# --- npm global packages ------------------------------------------------------
#
# Configure npm to install globals into ~/.npm-global so we never need sudo
# for `npm install -g`. This is the canonical Arch fix for permission errors.

write_step "Configuring npm global prefix to ~/.npm-global (no sudo required)"
NPM_PREFIX="$HOME/.npm-global"
mkdir -p "$NPM_PREFIX"
npm config set prefix "$NPM_PREFIX"
case ":$PATH:" in
    *":$NPM_PREFIX/bin:"*) ;;
    *) export PATH="$NPM_PREFIX/bin:$PATH" ;;
esac
write_ok "npm prefix: $NPM_PREFIX"
write_warn "Add this to your shell profile so npm globals stay on PATH across sessions:"
write_warn "  set -x PATH \$HOME/.npm-global/bin \$PATH        # fish"
write_warn "  export PATH=\"\$HOME/.npm-global/bin:\$PATH\"      # bash/zsh"

write_step "Installing wrangler (Cloudflare Workers CLI)"
if have wrangler; then
    write_skip "wrangler $(wrangler --version 2>/dev/null | head -n1)"
else
    npm install -g wrangler
    write_ok "wrangler"
fi

write_step "Installing @capacitor/cli"
if have cap; then
    write_skip "cap $(cap --version 2>/dev/null)"
else
    npm install -g @capacitor/cli
    write_ok "@capacitor/cli"
fi

# --- Summary ------------------------------------------------------------------

printf '\n'
printf '%s================================================================%s\n' "$C_MAGENTA" "$C_RESET"
printf '%s  TOOL INSTALLATION COMPLETE%s\n' "$C_MAGENTA" "$C_RESET"
printf '%s================================================================%s\n' "$C_MAGENTA" "$C_RESET"
printf '\n'

printf '%sInstalled tools -- verify versions:%s\n' "$C_WHITE" "$C_RESET"
print_tool() {
    local cmd=$1 arg=$2
    if have "$cmd"; then
        local ver
        ver=$("$cmd" $arg 2>/dev/null | head -n1)
        printf '  %s%-12s%s %s\n' "$C_GREEN" "$cmd" "$C_RESET" "$ver"
    else
        printf '  %s%-12s NOT FOUND (open a new shell?)%s\n' "$C_YELLOW" "$cmd" "$C_RESET"
    fi
}
print_tool git       --version
print_tool gh        --version
print_tool node      --version
print_tool npm       --version
print_tool rustc     --version
print_tool cargo     --version
print_tool sqlx      --version
print_tool flyctl    version
print_tool wrangler  --version
print_tool cap       --version

printf '\n'
printf '%s================================================================%s\n' "$C_YELLOW" "$C_RESET"
printf '%s  MANUAL STEPS (accounts + credentials -- cannot be scripted)%s\n' "$C_YELLOW" "$C_RESET"
printf '%s================================================================%s\n' "$C_YELLOW" "$C_RESET"
printf '\n'

cat <<EOF
  [ ] 1.  Android Studio -- launch and install SDK API 26+ + NDK (see warning above)
  [ ] 2.  Apple FamilyControls entitlement -- apply NOW (1-4 week lead time):
           https://developer.apple.com/contact/request/family-controls-distribution
  [ ] 3.  Cloudflare -- create R2 bucket, D1 database, KV namespace, Pages project
           https://dash.cloudflare.com
  [ ] 4.  Fly.io -- create account + run:  flyctl auth login
           https://fly.io
  [ ] 5.  Neon -- create project, copy DATABASE_URL to backend/.env
           https://console.neon.tech
  [ ] 6.  Firebase -- create project, enable FCM, download service-account.json
           https://console.firebase.google.com
  [ ] 7.  Discord Developer App -- create app, copy Client ID + Secret
           https://discord.com/developers/applications
  [ ] 8.  Google Cloud OAuth2 -- create credentials (Web application)
           https://console.cloud.google.com/apis/credentials
  [ ] 9.  Resend -- create account, get API key
           https://resend.com
  [ ] 10. Apple Developer Program (\$99/year) -- only needed at App Store submission
           https://developer.apple.com/programs
  [ ] 11. Google Play Console (\$25 one-time) -- only needed at Play Store submission
           https://play.google.com/console
  [ ] 12. Purchase domain on Porkbun (yapperhq.com or alternative)
           https://porkbun.com

  Once accounts are ready, copy credentials to:
    backend/.env   (DATABASE_URL, JWT_PRIVATE_KEY, JWT_PUBLIC_KEY, RESEND_API_KEY, ...)
    .github/       (FLY_API_TOKEN secret in repo settings)

EOF

printf '%sOpen a NEW terminal after this script so PATH changes take effect.%s\n\n' "$C_CYAN" "$C_RESET"
