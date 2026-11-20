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
tty() { (exec </dev/tty) 2>/dev/null; }
as_root() {
    if [ "$(id -u)" = 0 ]; then "$@"
    elif has sudo; then sudo "$@"
    elif has doas; then doas "$@"
    else say "root access is needed for: $*"; return 1; fi
}
ask() {
    [ "${KKRIEGER_YES:-}" = 1 ] && return 0
    tty || return 1
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
    # Package manager: install command and package names per distribution family.
    if has apt-get; then pm=apt; install_cmd="apt-get install -y"; libs="libx11-6 libxi6 libgl1 libasound2t64"
    elif has dnf; then pm=dnf; install_cmd="dnf install -y"; libs="libX11 libXi mesa-libGL alsa-lib"
    elif has pacman; then pm=pacman; install_cmd="pacman -S --needed --noconfirm"; libs="libx11 libxi libglvnd alsa-lib"
    elif has zypper; then pm=zypper; install_cmd="zypper --non-interactive install"; libs="libX11-6 libXi6 libGL1 libasound2"
    else pm=; install_cmd=; libs=; fi
    # shellcheck disable=SC2086 # command and package lists split into words
    pkg_install() {
        [ "$pm" = apt ] && { as_root apt-get update -qq || true; }
        as_root $install_cmd "$@"
    }
    if [ -n "$missing" ]; then
        say "missing runtime libraries:$missing"
        if [ -n "$pm" ] && ask "install $libs?"; then
            # shellcheck disable=SC2086
            pkg_install $libs ||
                { [ "$pm" = apt ] && pkg_install libx11-6 libxi6 libgl1 libasound2; } ||
                say "package install failed; install the libraries above manually"
        elif [ -n "$pm" ]; then
            say "install them with: $install_cmd $libs"
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
    # Original Microsoft core fonts installers, as listed by Debian's
    # ttf-mscorefonts-installer, with the checksums of the extracted faces.
    corefonts() {
        has cabextract || { [ -n "$pm" ] && pkg_install cabextract >/dev/null 2>&1; } || return 1
        mkdir -p "$home/fonts"
        while read -r cab cab_sum ttf ttf_sum name; do
            fetch "https://downloads.sourceforge.net/corefonts/$cab" "$tmp/$cab" || return 1
            [ "$(sha256 "$tmp/$cab")" = "$cab_sum" ] || { say "checksum mismatch for $cab"; return 1; }
            cabextract -q -L -d "$tmp" -F "$ttf" "$tmp/$cab" || return 1
            [ "$(sha256 "$tmp/$ttf")" = "$ttf_sum" ] || { say "checksum mismatch for $ttf"; return 1; }
            mv -f "$tmp/$ttf" "$home/fonts/$name"
        done <<'FONTS'
arial32.exe 85297a4d146e9c87ac6f74822734bdee5f4b2a722d7eaa584b7f2cbf76f478f6 arial.ttf 35c0f3559d8db569e36c31095b8a60d441643d95f59139de40e23fada819b833 Arial.ttf
times32.exe db56595ec6ef5d3de5c24994f001f03b2a13e37cee27bc25c58f6f43e8f807ab times.ttf 4e98adeff8ccc8ef4e3ece8d4547e288ff85fdc9c7ca711a4599c234874bbe86 Times New Roman.ttf
FONTS
    }
    if ! link_fonts; then
        say "Arial and Times New Roman are not installed"
        if [ "$pm" = apt ] && ask "install ttf-mscorefonts-installer (shows the Microsoft EULA)?"; then
            # sudo resets the environment; keep a caller-selected debconf frontend.
            frontend=${DEBIAN_FRONTEND:+DEBIAN_FRONTEND=$DEBIAN_FRONTEND}
            as_root apt-get update -qq || true
            if tty; then
                # shellcheck disable=SC2024,SC2086 # apt reads the EULA answer from the terminal
                as_root env $frontend apt-get install -y ttf-mscorefonts-installer </dev/tty || true
            else
                # shellcheck disable=SC2086
                as_root env $frontend apt-get install -y ttf-mscorefonts-installer || true
            fi
            link_fonts || true
        fi
        if [ ! -e "$home/fonts/Arial.ttf" ]; then
            say "the Microsoft core fonts EULA: https://corefonts.sourceforge.net/eula.htm"
            if ask "accept the EULA and download Arial and Times New Roman for kkrieger-oxide?"; then
                corefonts || say "font download failed"
            fi
        fi
        [ -e "$home/fonts/Arial.ttf" ] && [ -e "$home/fonts/Times New Roman.ttf" ] ||
            say "text needs Arial and Times New Roman; set KKRIEGER_FONT_DIR to their folder"
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
