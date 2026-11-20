#!/bin/sh
# kkrieger-oxide installer for macOS and Linux:
#   curl -fsSL https://github.com/fentz26/kkrieger-oxide/releases/latest/download/install.sh | sh
# Installs one compressed executable for this OS and CPU, verified by SHA-256.
# Linux also links the Arial and Times New Roman fonts the game rasterizes.
# Environment: KKRIEGER_BASE_URL, KKRIEGER_HOME (default ~/.local/share/kkrieger-oxide),
# KKRIEGER_BIN (default ~/.local/bin), KKRIEGER_YES=1 to accept prompts.
set -eu

base=${KKRIEGER_BASE_URL:-https://github.com/fentz26/kkrieger-oxide/releases/latest/download}
home=${KKRIEGER_HOME:-$HOME/.local/share/kkrieger-oxide}
bin=${KKRIEGER_BIN:-$HOME/.local/bin}

say() { printf 'kkrieger-oxide: %s\n' "$*" >&2; }
die() { say "$*"; exit 1; }
has() { command -v "$1" >/dev/null 2>&1; }
ask() {
    [ "${KKRIEGER_YES:-}" = 1 ] && return 0
    [ -r /dev/tty ] || return 1
    printf 'kkrieger-oxide: %s [y/N] ' "$1" >/dev/tty
    read -r reply </dev/tty || return 1
    case $reply in [Yy]*) return 0 ;; *) return 1 ;; esac
}
fetch() {
    if has curl; then curl -fsSL --retry 3 -o "$2" "$1"
    elif has wget; then wget -q -O "$2" "$1"
    else die "curl or wget is required"; fi
}
sha256() {
    if has sha256sum; then sha256sum "$1" | cut -d' ' -f1
    else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

os=$(uname -s)
case $(uname -m) in
    x86_64 | amd64) cpu=x86_64 ;;
    arm64 | aarch64) cpu=aarch64 ;;
    *) die "unsupported CPU $(uname -m)" ;;
esac
case $os in
    Darwin) target=$cpu-apple-darwin ;;
    Linux) target=$cpu-unknown-linux-gnu ;;
    *) die "unsupported OS $os (Windows: irm $base/install.ps1 | iex)" ;;
esac

# macOS decodes .xz with the system Compression framework; Linux uses xz or gzip.
if [ "$os" = Darwin ]; then
    ext=xz
    unpack() { compression_tool -decode -a lzma -i "$1" -o "$2"; }
elif has xz; then
    ext=xz
    unpack() { xz -dc "$1" >"$2"; }
else
    ext=gz
    unpack() { gzip -dc "$1" >"$2"; }
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
asset=kkrieger-oxide-$target.$ext
say "downloading $asset"
fetch "$base/$asset" "$tmp/$asset"
fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS"
expected=$(awk -v a="$asset" '$2 == a || $2 == "*" a { print $1 }' "$tmp/SHA256SUMS")
[ -n "$expected" ] || die "no checksum published for $asset"
[ "$(sha256 "$tmp/$asset")" = "$expected" ] || die "checksum mismatch for $asset"
unpack "$tmp/$asset" "$tmp/kkrieger-oxide"
chmod 755 "$tmp/kkrieger-oxide"
mkdir -p "$home" "$bin"
mv -f "$tmp/kkrieger-oxide" "$home/kkrieger-oxide"

if [ "$os" = Linux ]; then
    # Runtime libraries loaded with dlopen by the window, GL and audio layers.
    missing=
    for lib in libX11.so.6 libXi.so.6 libGL.so.1 libasound.so.2; do
        if ! { has ldconfig && ldconfig -p 2>/dev/null | grep -q "$lib"; } &&
            ! ls /usr/lib/$lib /usr/lib64/$lib /usr/lib/*-linux-gnu/$lib >/dev/null 2>&1; then
            missing="$missing $lib"
        fi
    done
    if [ -n "$missing" ]; then
        if has apt-get; then pkgs="libx11-6 libxi6 libgl1 libasound2t64"; inst="apt-get install -y"; sudo_update() { sudo apt-get update -qq; }
        elif has dnf; then pkgs="libX11 libXi mesa-libGL alsa-lib"; inst="dnf install -y"; sudo_update() { :; }
        elif has pacman; then pkgs="libx11 libxi libglvnd alsa-lib"; inst="pacman -S --needed --noconfirm"; sudo_update() { :; }
        elif has zypper; then pkgs="libX11-6 libXi6 libGL1 libasound2"; inst="zypper install -y"; sudo_update() { :; }
        else pkgs=; inst=; fi
        say "missing runtime libraries:$missing"
        if [ -n "$inst" ] && ask "install $pkgs with sudo?"; then
            # Debian renamed libasound2 to libasound2t64; fall back to the older name.
            sudo_update || true
            # shellcheck disable=SC2086 # command and package lists split into words
            sudo $inst $pkgs || { [ "${inst%% *}" = apt-get ] && sudo $inst libx11-6 libxi6 libgl1 libasound2; } ||
                say "package install failed; install the libraries above manually"
        elif [ -n "$inst" ]; then
            say "install them with: sudo $inst $pkgs"
        fi
    fi

    # The game rasterizes the original Arial and Times New Roman faces.
    font() {
        for name in "$@"; do
            path=
            if has fc-list; then
                path=$(fc-list : file 2>/dev/null | sed 's/: *$//' | grep -i "/$name\$" | head -n 1)
            fi
            if [ -z "$path" ]; then
                path=$(find /usr/share/fonts /usr/local/share/fonts "$HOME/.local/share/fonts" "$HOME/.fonts" \
                    -iname "$name" 2>/dev/null | head -n 1)
            fi
            [ -n "$path" ] && { printf '%s\n' "$path"; return 0; }
        done
        return 1
    }
    link_fonts() {
        arial=$(font Arial.ttf arial.ttf) && times=$(font 'Times New Roman.ttf' times.ttf Times_New_Roman.ttf) || return 1
        mkdir -p "$home/fonts"
        ln -sf "$arial" "$home/fonts/Arial.ttf"
        ln -sf "$times" "$home/fonts/Times New Roman.ttf"
    }
    if ! link_fonts; then
        say "Arial and Times New Roman are not installed"
        if has apt-get && ask "install ttf-mscorefonts-installer with sudo (shows the Microsoft EULA)?"; then
            sudo apt-get update -qq || true
            # The package asks the player to accept the EULA on the terminal,
            # not on the piped script.
            if [ -r /dev/tty ]; then
                sudo apt-get install -y ttf-mscorefonts-installer </dev/tty || true
            else
                sudo apt-get install -y ttf-mscorefonts-installer || true
            fi
        elif has dnf; then
            say "install the Microsoft core fonts (msttcore-fonts), then run this installer again"
        elif has pacman; then
            say "install ttf-ms-fonts from the AUR, then run this installer again"
        fi
        link_fonts || say "text needs Arial and Times New Roman; set KKRIEGER_FONT_DIR to their folder"
    fi
    cat >"$bin/kkrieger-oxide" <<EOF
#!/bin/sh
[ -d "$home/fonts" ] && : "\${KKRIEGER_FONT_DIR:=$home/fonts}" && export KKRIEGER_FONT_DIR
exec "$home/kkrieger-oxide" "\$@"
EOF
    chmod 755 "$bin/kkrieger-oxide"
else
    ln -sf "$home/kkrieger-oxide" "$bin/kkrieger-oxide"
fi

say "installed $home/kkrieger-oxide"
case :$PATH: in
    *:"$bin":*) say "run: kkrieger-oxide" ;;
    *) say "run: $bin/kkrieger-oxide (add $bin to PATH to run kkrieger-oxide)" ;;
esac
