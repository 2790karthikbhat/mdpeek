# Install the latest mdpeek release on Windows. Non-interactive, safe to re-run.
#
#   irm https://raw.githubusercontent.com/2790karthikbhat/mdpeek/main/scripts/install.ps1 | iex
#
# Downloads the prebuilt binary, checks its SHA-256, installs it to
# %LOCALAPPDATA%\Programs\mdpeek and adds that folder to the user PATH.
# Options (environment variables): MDPEEK_VERSION=v0.1.0, MDPEEK_INSTALL_DIR=C:\path

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repo = '2790karthikbhat/mdpeek'
$version = if ($env:MDPEEK_VERSION) { $env:MDPEEK_VERSION } else { 'latest' }
$dir = if ($env:MDPEEK_INSTALL_DIR) { $env:MDPEEK_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\mdpeek' }

if (-not [Environment]::Is64BitOperatingSystem) {
    throw 'Only 64-bit Windows builds are published. Install with Rust instead: cargo install mdpeek'
}
$target = 'x86_64-pc-windows-msvc'  # also runs on Windows on Arm through emulation

$base = if ($version -eq 'latest') { "https://github.com/$repo/releases/latest/download" }
        else { "https://github.com/$repo/releases/download/$version" }
$asset = "mdpeek-$target.zip"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("mdpeek-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null

try {
    Write-Host "Downloading $base/$asset"
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile (Join-Path $tmp $asset)
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset.sha256" -OutFile (Join-Path $tmp "$asset.sha256")

    $expected = ((Get-Content (Join-Path $tmp "$asset.sha256") -Raw).Trim() -split '\s+')[0].ToLower()
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $asset)).Hash.ToLower()
    if ($expected -ne $actual) { throw "Checksum mismatch for $asset (expected $expected, got $actual)" }

    Expand-Archive -Path (Join-Path $tmp $asset) -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Copy-Item (Join-Path $tmp "mdpeek-$target\mdpeek.exe") (Join-Path $dir 'mdpeek.exe') -Force

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not (($userPath -split ';') -contains $dir)) {
        [Environment]::SetEnvironmentVariable('Path', ($userPath.TrimEnd(';') + ";$dir").TrimStart(';'), 'User')
        $env:Path += ";$dir"
        Write-Host "Added $dir to your user PATH (open a new terminal to pick it up)."
    }
    $v = & (Join-Path $dir 'mdpeek.exe') --version
    Write-Host "Installed $v to $dir\mdpeek.exe"
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
