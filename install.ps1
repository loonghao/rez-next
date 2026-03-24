# rez-next installer for Windows
# Usage: irm https://raw.githubusercontent.com/loonghao/rez-next/main/install.ps1 | iex
#
# Environment variables:
#   REZ_NEXT_VERSION  - Specific version to install (e.g., "0.1.0"). Default: latest
#   REZ_NEXT_INSTALL  - Installation directory. Default: $HOME\.rez-next\bin
#   REZ_NEXT_NO_PATH  - Set to "1" to skip adding to PATH. Default: auto-add

$ErrorActionPreference = 'Stop'
$Repo = "loonghao/rez-next"
$BinaryName = "rez-next"

function Write-Info { param([string]$Message); Write-Host "info: " -ForegroundColor Blue -NoNewline; Write-Host $Message }
function Write-Success { param([string]$Message); Write-Host "success: " -ForegroundColor Green -NoNewline; Write-Host $Message }
function Write-Warn { param([string]$Message); Write-Host "warn: " -ForegroundColor Yellow -NoNewline; Write-Host $Message }
function Write-Err { param([string]$Message); Write-Host "error: " -ForegroundColor Red -NoNewline; Write-Host $Message; exit 1 }

function Get-Architecture {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($arch) {
        "X64" { return "x86_64" }
        "Arm64" { return "aarch64" }
        default { Write-Err "Unsupported architecture: $arch" }
    }
}

function Get-LatestVersion {
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        return $release.tag_name -replace '^v', ''
    } catch {
        Write-Err "Failed to fetch latest version: $_"
    }
}

function Install-RezNext {
    $arch = Get-Architecture
    $target = "${arch}-pc-windows-msvc"
    Write-Info "Detected platform: windows $arch ($target)"

    # Determine version
    $version = $env:REZ_NEXT_VERSION
    if ([string]::IsNullOrEmpty($version) -or $version -eq "latest") {
        Write-Info "Fetching latest version..."
        $version = Get-LatestVersion
    }
    if ([string]::IsNullOrEmpty($version)) {
        Write-Err "Failed to determine version. Set REZ_NEXT_VERSION or check your network connection."
    }
    Write-Info "Installing rez-next v$version..."

    # Determine install directory
    $installDir = $env:REZ_NEXT_INSTALL
    if ([string]::IsNullOrEmpty($installDir)) {
        $installDir = Join-Path $HOME ".rez-next\bin"
    }
    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    # Check for existing installation
    $oldVersion = $null
    $existingBinary = Join-Path $installDir "$BinaryName.exe"
    if (Test-Path $existingBinary) {
        try {
            $oldVersionOutput = & $existingBinary --version 2>&1 | Select-Object -First 1
            $oldVersion = ($oldVersionOutput -split '\s+')[1]
            if ($oldVersion) {
                Write-Info "Found existing installation: rez-next v$oldVersion"
            }
        } catch {
            # Ignore errors from old binary
        }
    }

    # Create temp directory
    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "rez-next-install-$([System.Guid]::NewGuid().ToString('N').Substring(0,8))"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        $archiveName = "$BinaryName-$target.zip"
        $downloadUrl = "https://github.com/$Repo/releases/download/v$version/$archiveName"
        $archivePath = Join-Path $tmpDir $archiveName

        Write-Info "Downloading $downloadUrl..."
        try {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
        } catch {
            Write-Err "Download failed. Check if v$version has pre-built binaries for $target. Error: $_"
        }

        # Verify SHA256 checksum if available
        $checksumsUrl = "https://github.com/$Repo/releases/download/v$version/checksums-sha256.txt"
        $checksumsPath = Join-Path $tmpDir "checksums-sha256.txt"
        try {
            Invoke-WebRequest -Uri $checksumsUrl -OutFile $checksumsPath -UseBasicParsing -ErrorAction Stop
            Write-Info "Verifying SHA256 checksum..."
            $checksumLines = Get-Content $checksumsPath
            $expectedLine = $checksumLines | Where-Object { $_ -match [regex]::Escape($archiveName) }
            if ($expectedLine) {
                # Support both standard "hash  filename" and legacy "hashfilename" formats
                if ($expectedLine -match '^\s*([0-9a-fA-F]{64})\s+') {
                    $expectedHash = $Matches[1].ToLower()
                } else {
                    # Legacy format: hash directly concatenated with filename
                    $expectedHash = ($expectedLine -replace [regex]::Escape($archiveName), '').Trim().ToLower()
                }
                $actualHash = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash.ToLower()
                if ($actualHash -eq $expectedHash) {
                    Write-Success "Checksum verified ✓"
                } else {
                    Write-Err "Checksum mismatch! Expected: $expectedHash, Got: $actualHash"
                }
            } else {
                Write-Warn "Checksum not found for $archiveName, skipping verification"
            }
        } catch {
            Write-Warn "Checksums file not available, skipping verification"
        }

        # Extract
        Write-Info "Extracting..."
        Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force

        # Find binary
        $binaryPath = Get-ChildItem -Path $tmpDir -Recurse -Filter "$BinaryName.exe" | Select-Object -First 1
        if ($null -eq $binaryPath) {
            Write-Err "Could not find $BinaryName.exe in the downloaded archive"
        }

        # Install
        $destPath = Join-Path $installDir "$BinaryName.exe"
        Copy-Item -Path $binaryPath.FullName -Destination $destPath -Force
        Write-Success "rez-next v$version installed to $destPath"

        # Show upgrade info if applicable
        if ($oldVersion -and $oldVersion -ne $version) {
            Write-Success "Upgraded: v$oldVersion → v$version"
        }

        # Check if install_dir is in PATH
        $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$installDir*") {
            if ($env:REZ_NEXT_NO_PATH -eq "1") {
                Write-Host ""
                Write-Warn "'$installDir' is not in your PATH."
                Write-Host ""
                Write-Host " To add it permanently, run:" -ForegroundColor Gray
                Write-Host ""
                Write-Host " [Environment]::SetEnvironmentVariable('Path', `"$installDir;`" + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')" -ForegroundColor Cyan
                Write-Host ""
            } else {
                [System.Environment]::SetEnvironmentVariable("Path", "$installDir;" + $userPath, "User")
                $env:Path = "$installDir;$env:Path"
                Write-Success "Added $installDir to user PATH"
            }
        }

        # Verify installation
        try {
            $installedVersion = & $destPath --version 2>&1 | Select-Object -First 1
            Write-Success "Verified: $installedVersion"
        } catch {
            Write-Warn "Could not verify installation, but binary was placed at $destPath"
        }

        Write-Host ""
        Write-Info "Run 'rez-next --help' to get started"
    } finally {
        # Cleanup
        if (Test-Path $tmpDir) {
            Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Install-RezNext
