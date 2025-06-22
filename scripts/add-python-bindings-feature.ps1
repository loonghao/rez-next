#!/usr/bin/env pwsh
# Script to add python-bindings feature to all crates

param(
    [switch]$DryRun,
    [switch]$Help
)

if ($Help) {
    Write-Host @"
Add python-bindings feature to all crates

Usage: ./scripts/add-python-bindings-feature.ps1 [OPTIONS]

Options:
    --dry-run    Show what would be changed without making changes
    --help       Show this help message
"@
    exit 0
}

$RootDir = Split-Path -Parent $PSScriptRoot

Write-Host "🔧 Adding python-bindings feature to all crates..." -ForegroundColor Green
Write-Host "Dry run: $DryRun" -ForegroundColor Cyan
Write-Host ""

# Find all crate Cargo.toml files
$crateFiles = Get-ChildItem -Path "$RootDir/crates" -Recurse -Name "Cargo.toml" | ForEach-Object {
    Join-Path "$RootDir/crates" $_
}

$updatedCount = 0

foreach ($cargoFile in $crateFiles) {
    Write-Host "Processing: $cargoFile" -ForegroundColor Cyan
    
    if (-not (Test-Path $cargoFile)) {
        continue
    }
    
    $content = Get-Content $cargoFile -Raw
    $originalContent = $content
    
    # Check if [features] section exists
    if ($content -match '\[features\]') {
        # Check if python-bindings feature already exists
        if ($content -notmatch 'python-bindings\s*=') {
            # Add python-bindings feature after default = []
            $content = $content -replace '(\[features\]\s*\r?\n\s*default\s*=\s*\[\])', "`$1`r`n# Python bindings feature (defined to avoid warnings)`r`npython-bindings = []  # No dependencies for now"
            Write-Host "  ✅ Added python-bindings feature" -ForegroundColor Green
        } else {
            Write-Host "  ⏭️ python-bindings feature already exists" -ForegroundColor Yellow
        }
    } else {
        # Add entire [features] section before [lib] or [dependencies]
        $featuresSection = @"
[features]
default = []
# Python bindings feature (defined to avoid warnings)
python-bindings = []  # No dependencies for now

"@
        
        if ($content -match '\[lib\]') {
            $content = $content -replace '(\[lib\])', "$featuresSection`$1"
            Write-Host "  ✅ Added [features] section before [lib]" -ForegroundColor Green
        } elseif ($content -match '\[dependencies\]') {
            $content = $content -replace '(\[dependencies\])', "$featuresSection`$1"
            Write-Host "  ✅ Added [features] section before [dependencies]" -ForegroundColor Green
        } else {
            # Add at the end
            $content = $content + "`r`n$featuresSection"
            Write-Host "  ✅ Added [features] section at the end" -ForegroundColor Green
        }
    }
    
    if ($content -ne $originalContent) {
        if (-not $DryRun) {
            Set-Content -Path $cargoFile -Value $content -NoNewline
        }
        $updatedCount++
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
    Write-Host "✅ python-bindings features added successfully!" -ForegroundColor Green
}
