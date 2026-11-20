# kkrieger-oxide installer for Windows:
#   irm https://github.com/fentz26/kkrieger-oxide/releases/latest/download/install.ps1 | iex
# Installs one compressed executable, verified by SHA-256, plus a Start menu entry.
# Windows provides the D3D, WinMM and GDI font services the game uses.
# Environment: KKRIEGER_BASE_URL, KKRIEGER_HOME (default %LOCALAPPDATA%\Programs\kkrieger-oxide).
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$base = if ($env:KKRIEGER_BASE_URL) { $env:KKRIEGER_BASE_URL } else { 'https://github.com/fentz26/kkrieger-oxide/releases/latest/download' }
$home_ = if ($env:KKRIEGER_HOME) { $env:KKRIEGER_HOME } else { Join-Path $env:LOCALAPPDATA 'Programs\kkrieger-oxide' }
# x64 executables also run on Windows on Arm through emulation.
$asset = 'kkrieger-oxide-x86_64-pc-windows-msvc.exe.gz'
$tmp = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Write-Host "kkrieger-oxide: downloading $asset"
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile (Join-Path $tmp $asset)
    Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile (Join-Path $tmp 'SHA256SUMS')
    $expected = (Get-Content (Join-Path $tmp 'SHA256SUMS') | Where-Object { ($_ -split '\s+')[1] -in @($asset, "*$asset") } | ForEach-Object { ($_ -split '\s+')[0] }) | Select-Object -First 1
    if (-not $expected) { throw "no checksum published for $asset" }
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $asset)).Hash
    if ($actual -ne $expected) { throw "checksum mismatch for $asset" }
    New-Item -ItemType Directory -Force -Path $home_ | Out-Null
    $exe = Join-Path $home_ 'kkrieger-oxide.exe'
    $source = [IO.File]::OpenRead((Join-Path $tmp $asset))
    try {
        $gzip = New-Object IO.Compression.GZipStream($source, [IO.Compression.CompressionMode]::Decompress)
        $target = [IO.File]::Create($exe)
        try { $gzip.CopyTo($target) } finally { $target.Dispose(); $gzip.Dispose() }
    } finally { $source.Dispose() }
    $menu = Join-Path ([Environment]::GetFolderPath('Programs')) 'kkrieger-oxide.lnk'
    $link = (New-Object -ComObject WScript.Shell).CreateShortcut($menu)
    $link.TargetPath = $exe
    $link.WorkingDirectory = $home_
    $link.Save()
    Write-Host "kkrieger-oxide: installed $exe"
    Write-Host "kkrieger-oxide: run it from the Start menu or with: & '$exe'"
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
