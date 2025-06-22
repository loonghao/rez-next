# Rez-Core Comprehensive Demo Script
# This script demonstrates all the major features of rez-core

Write-Host "🚀 Rez-Core Comprehensive Demo" -ForegroundColor Green
Write-Host "=============================" -ForegroundColor Green
Write-Host ""

$REZ_EXE = ".\target\debug\rez.exe"

# Check if rez.exe exists
if (-not (Test-Path $REZ_EXE)) {
    Write-Host "❌ Error: rez.exe not found at $REZ_EXE" -ForegroundColor Red
    Write-Host "Please run 'cargo build --bin rez' first" -ForegroundColor Yellow
    exit 1
}

Write-Host "📋 1. Basic Information" -ForegroundColor Cyan
Write-Host "----------------------" -ForegroundColor Cyan
& $REZ_EXE --version
& $REZ_EXE --help | Select-Object -First 10
Write-Host ""

Write-Host "🔍 2. Package Search" -ForegroundColor Cyan
Write-Host "-------------------" -ForegroundColor Cyan
Write-Host "Searching for 'python' packages:"
& $REZ_EXE search python
Write-Host ""

Write-Host "Advanced search with regex:"
& $REZ_EXE search "py.*" --regex --verbose
Write-Host ""

Write-Host "🧩 3. Dependency Resolution" -ForegroundColor Cyan
Write-Host "---------------------------" -ForegroundColor Cyan
Write-Host "Resolving dependencies for scipy (includes python and numpy):"
& $REZ_EXE solve scipy --repository C:\temp\test-packages --verbose
Write-Host ""

Write-Host "JSON output format:"
& $REZ_EXE solve python numpy --repository C:\temp\test-packages --format json
Write-Host ""

Write-Host "📦 4. Context Management" -ForegroundColor Cyan
Write-Host "-----------------------" -ForegroundColor Cyan
Write-Host "Context summary:"
& $REZ_EXE context
Write-Host ""

Write-Host "Available tools:"
& $REZ_EXE context --tools
Write-Host ""

Write-Host "Shell environment (Bash):"
& $REZ_EXE context --interpret --format bash
Write-Host ""

Write-Host "Shell environment (PowerShell):"
& $REZ_EXE context --interpret --format power-shell
Write-Host ""

Write-Host "🔧 5. Version Parsing" -ForegroundColor Cyan
Write-Host "--------------------" -ForegroundColor Cyan
Write-Host "Testing version parsing:"
& $REZ_EXE parse-version "1.2.3"
& $REZ_EXE parse-version "2.0.0-alpha1"
& $REZ_EXE parse-version "3.1.4-beta.2"
Write-Host ""

Write-Host "🧪 6. Self Tests" -ForegroundColor Cyan
Write-Host "---------------" -ForegroundColor Cyan
Write-Host "Running internal tests:"
& $REZ_EXE self-test
Write-Host ""

Write-Host "🏗️ 7. Build System" -ForegroundColor Cyan
Write-Host "-----------------" -ForegroundColor Cyan
Write-Host "Build command help:"
& $REZ_EXE build --help | Select-Object -First 15
Write-Host ""

Write-Host "✅ Demo Complete!" -ForegroundColor Green
Write-Host "=================" -ForegroundColor Green
Write-Host ""
Write-Host "🎯 Key Features Demonstrated:" -ForegroundColor Yellow
Write-Host "  • Advanced package search with regex and filtering" -ForegroundColor White
Write-Host "  • Intelligent dependency resolution" -ForegroundColor White
Write-Host "  • Context management and environment generation" -ForegroundColor White
Write-Host "  • Multiple output formats (table, JSON, shell scripts)" -ForegroundColor White
Write-Host "  • Version parsing and validation" -ForegroundColor White
Write-Host "  • Network build support (git repositories, archives)" -ForegroundColor White
Write-Host "  • Cross-platform shell support (Bash, PowerShell, Fish, etc.)" -ForegroundColor White
Write-Host ""
Write-Host "🚀 Performance Benefits:" -ForegroundColor Yellow
Write-Host "  • Written in Rust for maximum performance" -ForegroundColor White
Write-Host "  • Async I/O for fast repository scanning" -ForegroundColor White
Write-Host "  • Intelligent caching for repeated operations" -ForegroundColor White
Write-Host "  • Optimized dependency resolution algorithms" -ForegroundColor White
Write-Host ""
Write-Host "📚 Next Steps:" -ForegroundColor Yellow
Write-Host "  • Try: rez-core search <your-package>" -ForegroundColor White
Write-Host "  • Try: rez-core solve <requirements>" -ForegroundColor White
Write-Host "  • Try: rez-core build <source-url>" -ForegroundColor White
Write-Host "  • Try: rez-core context --interpret --format <shell>" -ForegroundColor White
Write-Host ""
