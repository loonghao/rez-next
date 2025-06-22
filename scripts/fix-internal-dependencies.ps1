#!/usr/bin/env pwsh
# Script to add version numbers to internal dependencies for crates.io publishing

param(
    [switch]$DryRun,
    [switch]$Help
)

if ($Help) {
    Write-Host @"
Add version numbers to internal dependencies for crates.io publishing

Usage: ./scripts/fix-internal-dependencies.ps1 [OPTIONS]

Options:
    --dry-run    Show what would be changed without making changes
    --help       Show this help message
"@
    exit 0
}

$RootDir = Split-Path -Parent $PSScriptRoot

Write-Host "🔧 Adding version numbers to internal dependencies..." -ForegroundColor Green
Write-Host "Dry run: $DryRun" -ForegroundColor Cyan
Write-Host ""

# Define internal crates and their version
$Version = "0.1.0"
$InternalCrates = @(
    "rez-next-common",
    "rez-next-version", 
    "rez-next-package",
    "rez-next-repository",
    "rez-next-solver",
    "rez-next-context",
    "rez-next-build",
    "rez-next-cache"
)

# Find all crate Cargo.toml files
$crateFiles = Get-ChildItem -Path "$RootDir/crates" -Recurse -Name "Cargo.toml" | ForEach-Object {
    Join-Path "$RootDir/crates" $_
}

# Also include the main Cargo.toml
$crateFiles += Join-Path $RootDir "Cargo.toml"

$updatedCount = 0

foreach ($cargoFile in $crateFiles) {
    Write-Host "Processing: $cargoFile" -ForegroundColor Cyan
    
    if (-not (Test-Path $cargoFile)) {
        continue
    }
    
    $content = Get-Content $cargoFile -Raw
    $originalContent = $content
    $fileUpdated = $false
    
    foreach ($crate in $InternalCrates) {
        # Pattern to match internal dependency without version
        $pattern = "($crate)\s*=\s*\{\s*path\s*="
        $replacement = "`$1 = { version = `"$Version`", path ="
        
        if ($content -match $pattern) {
            $content = $content -replace $pattern, $replacement
            Write-Host "  ✅ Added version to $crate dependency" -ForegroundColor Green
            $fileUpdated = $true
        }
    }
    
    if ($fileUpdated -and $content -ne $originalContent) {
        if (-not $DryRun) {
            Set-Content -Path $cargoFile -Value $content -NoNewline
        }
        $updatedCount++
    } else {
        Write-Host "  ⏭️ No internal dependencies found or already have versions" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "📊 Summary:" -ForegroundColor Green
Write-Host "  📝 Files updated: $updatedCount" -ForegroundColor Cyan
Write-Host "  📁 Total files checked: $($crateFiles.Count)" -ForegroundColor Cyan

if ($DryRun) {
    Write-Host ""
    Write-Host "🔍 This was a dry run. No actual changes were made." -ForegroundColor Yellow
    Write-Host "Run without --dry-run to apply the changes." -ForegroundColor Yellow
} else {
    Write-Host ""
    Write-Host "✅ Internal dependency versions added successfully!" -ForegroundColor Green
}
