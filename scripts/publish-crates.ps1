#!/usr/bin/env pwsh
# Batch publish script for rez-next crates
# Usage: ./scripts/publish-crates.ps1 [--dry-run] [--version <version>]

param(
    [switch]$DryRun,
    [string]$Version = "0.1.0",
    [switch]$Help
)

if ($Help) {
    Write-Host @"
Batch publish script for rez-next crates

Usage: ./scripts/publish-crates.ps1 [OPTIONS]

Options:
    --dry-run           Show what would be published without actually publishing
    --version <version> Set version for all crates (default: 0.1.0)
    --help             Show this help message

Examples:
    ./scripts/publish-crates.ps1 --dry-run
    ./scripts/publish-crates.ps1 --version 0.2.0
    ./scripts/publish-crates.ps1
"@
    exit 0
}

# Define the publishing order (dependencies first)
$PublishOrder = @(
    "rez-next-common",
    "rez-next-version", 
    "rez-next-package",
    "rez-next-repository",
    "rez-next-solver",
    "rez-next-context",
    "rez-next-build",
    "rez-next-cache"
)

$RootDir = Split-Path -Parent $PSScriptRoot
$CratesDir = Join-Path $RootDir "crates"

Write-Host "🚀 Starting rez-next crates publishing process" -ForegroundColor Green
Write-Host "Version: $Version" -ForegroundColor Cyan
Write-Host "Dry run: $DryRun" -ForegroundColor Cyan
Write-Host ""

# Function to check if crate exists on crates.io
function Test-CrateExists {
    param([string]$CrateName, [string]$Version)
    
    try {
        $response = Invoke-RestMethod -Uri "https://crates.io/api/v1/crates/$CrateName/$Version" -Method Get -ErrorAction SilentlyContinue
        return $true
    }
    catch {
        return $false
    }
}

# Function to publish a single crate
function Publish-Crate {
    param([string]$CrateName)
    
    $CratePath = Join-Path $CratesDir $CrateName
    
    if (-not (Test-Path $CratePath)) {
        Write-Host "❌ Crate directory not found: $CratePath" -ForegroundColor Red
        return $false
    }
    
    Write-Host "📦 Processing $CrateName..." -ForegroundColor Yellow
    
    # Check if version already exists
    if (Test-CrateExists -CrateName $CrateName -Version $Version) {
        Write-Host "⚠️  Version $Version already exists for $CrateName, skipping..." -ForegroundColor Yellow
        return $true
    }
    
    Push-Location $CratePath
    try {
        # Run tests first
        Write-Host "  🧪 Running tests..." -ForegroundColor Cyan
        if ($DryRun) {
            Write-Host "  [DRY RUN] Would run: cargo test" -ForegroundColor Gray
        } else {
            $testResult = cargo test --quiet
            if ($LASTEXITCODE -ne 0) {
                Write-Host "  ❌ Tests failed for $CrateName" -ForegroundColor Red
                return $false
            }
        }
        
        # Check package
        Write-Host "  📋 Checking package..." -ForegroundColor Cyan
        if ($DryRun) {
            Write-Host "  [DRY RUN] Would run: cargo check" -ForegroundColor Gray
        } else {
            $checkResult = cargo check --quiet
            if ($LASTEXITCODE -ne 0) {
                Write-Host "  ❌ Package check failed for $CrateName" -ForegroundColor Red
                return $false
            }
        }
        
        # Publish
        Write-Host "  🚀 Publishing..." -ForegroundColor Cyan
        if ($DryRun) {
            Write-Host "  [DRY RUN] Would run: cargo publish" -ForegroundColor Gray
        } else {
            $publishResult = cargo publish
            if ($LASTEXITCODE -ne 0) {
                Write-Host "  ❌ Publishing failed for $CrateName" -ForegroundColor Red
                return $false
            }
            
            # Wait a bit for crates.io to process
            Write-Host "  ⏳ Waiting for crates.io to process..." -ForegroundColor Cyan
            Start-Sleep -Seconds 30
        }
        
        Write-Host "  ✅ Successfully processed $CrateName" -ForegroundColor Green
        return $true
    }
    finally {
        Pop-Location
    }
}

# Main publishing loop
$SuccessCount = 0
$FailureCount = 0
$SkippedCount = 0

foreach ($CrateName in $PublishOrder) {
    $result = Publish-Crate -CrateName $CrateName
    
    if ($result) {
        $SuccessCount++
    } else {
        $FailureCount++
        Write-Host "❌ Failed to publish $CrateName" -ForegroundColor Red
        
        # Ask if we should continue
        if (-not $DryRun) {
            $continue = Read-Host "Continue with remaining crates? (y/N)"
            if ($continue -ne "y" -and $continue -ne "Y") {
                Write-Host "🛑 Publishing stopped by user" -ForegroundColor Red
                break
            }
        }
    }
}

# Summary
Write-Host ""
Write-Host "📊 Publishing Summary:" -ForegroundColor Green
Write-Host "  ✅ Successful: $SuccessCount" -ForegroundColor Green
Write-Host "  ❌ Failed: $FailureCount" -ForegroundColor Red
Write-Host "  📦 Total: $($PublishOrder.Count)" -ForegroundColor Cyan

if ($DryRun) {
    Write-Host ""
    Write-Host "🔍 This was a dry run. No actual publishing occurred." -ForegroundColor Yellow
    Write-Host "Run without --dry-run to actually publish the crates." -ForegroundColor Yellow
}

if ($FailureCount -eq 0) {
    Write-Host ""
    Write-Host "🎉 All crates processed successfully!" -ForegroundColor Green
    exit 0
} else {
    Write-Host ""
    Write-Host "⚠️  Some crates failed to publish. Check the output above." -ForegroundColor Yellow
    exit 1
}
