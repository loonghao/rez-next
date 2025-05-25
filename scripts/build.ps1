# Build script for rez-core on Windows
# Usage: .\scripts\build.ps1 [command]

param(
    [string]$Command = "build-dev"
)

$ErrorActionPreference = "Stop"

function Test-UV {
    try {
        uv -V | Out-Null
        return $true
    } catch {
        Write-Error "Please install uv: https://docs.astral.sh/uv/getting-started/installation/"
        return $false
    }
}

function Build-Dev {
    Write-Host "Building development version..." -ForegroundColor Green
    Remove-Item -Path "python/rez_core/*.so" -Force -ErrorAction SilentlyContinue
    uv run maturin develop --uv
}

function Build-Prod {
    Write-Host "Building production version..." -ForegroundColor Green
    Remove-Item -Path "python/rez_core/*.so" -Force -ErrorAction SilentlyContinue
    uv run maturin develop --uv --release
}

function Build-Wheel {
    Write-Host "Building wheel..." -ForegroundColor Green
    uv run maturin build --release
}

function Build-Profiling {
    Write-Host "Building profiling version..." -ForegroundColor Green
    Remove-Item -Path "python/rez_core/*.so" -Force -ErrorAction SilentlyContinue
    uv run maturin develop --uv --profile profiling
}

function Test-Python {
    Write-Host "Running Python tests..." -ForegroundColor Green
    uv run pytest tests/python/
}

function Test-Rust {
    Write-Host "Running Rust tests..." -ForegroundColor Green
    $env:PYTHONPATH = (uv run python -c "import sys; print(';'.join(sys.path))")
    $env:PYO3_PYTHON = (uv run python -c "import sys; print(sys.executable)")
    cargo test
}

function Test-All {
    Test-Python
    Test-Rust
}

function Lint-Python {
    Write-Host "Linting Python code..." -ForegroundColor Green
    uv run ruff check python/rez_core tests/python
    uv run ruff format --check python/rez_core tests/python
}

function Lint-Rust {
    Write-Host "Linting Rust code..." -ForegroundColor Green
    cargo fmt --all -- --check
    cargo clippy --tests -- -D warnings
}

function Lint-All {
    Lint-Python
    Lint-Rust
}

function Format-Code {
    Write-Host "Formatting code..." -ForegroundColor Green
    uv run ruff check --fix python/rez_core tests/python
    uv run ruff format python/rez_core tests/python
    cargo fmt
}

function Run-Benchmark {
    Write-Host "Running benchmarks..." -ForegroundColor Green
    uv run pytest tests/python/ -m performance --benchmark-enable
}

function Run-Bench-Rust {
    Write-Host "Running Rust benchmarks..." -ForegroundColor Green
    $env:PYTHONPATH = (uv run python -c "import sys; print(';'.join(sys.path))")
    $env:PYO3_PYTHON = (uv run python -c "import sys; print(sys.executable)")
    cargo bench
}

function Run-Flamegraph {
    Write-Host "Running flamegraph profiling..." -ForegroundColor Green

    # Check if flamegraph is installed
    try {
        flamegraph --version | Out-Null
    } catch {
        Write-Host "Installing flamegraph..." -ForegroundColor Yellow
        cargo install flamegraph
    }

    # Build with profiling symbols
    Build-Profiling

    Write-Host "Running flamegraph on benchmarks..." -ForegroundColor Green
    $env:PYTHONPATH = (uv run python -c "import sys; print(';'.join(sys.path))")
    $env:PYO3_PYTHON = (uv run python -c "import sys; print(sys.executable)")

    # Run flamegraph with criterion benchmarks
    flamegraph --output flamegraph.svg -- cargo bench --features flamegraph

    Write-Host "Flamegraph saved to flamegraph.svg" -ForegroundColor Green
}

function Clean-All {
    Write-Host "Cleaning build artifacts..." -ForegroundColor Green
    Remove-Item -Path "__pycache__" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "*.pyc" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path ".pytest_cache" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "htmlcov" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "target" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "python/rez_core/*.so" -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "*.egg-info" -Recurse -Force -ErrorAction SilentlyContinue
}

# Main execution
if (-not (Test-UV)) {
    exit 1
}

switch ($Command) {
    "build-dev" { Build-Dev }
    "build-prod" { Build-Prod }
    "build-profiling" { Build-Profiling }
    "build-wheel" { Build-Wheel }
    "test-python" { Test-Python }
    "test-rust" { Test-Rust }
    "test" { Test-All }
    "lint-python" { Lint-Python }
    "lint-rust" { Lint-Rust }
    "lint" { Lint-All }
    "format" { Format-Code }
    "benchmark" { Run-Benchmark }
    "bench-rust" { Run-Bench-Rust }
    "flamegraph" { Run-Flamegraph }
    "clean" { Clean-All }
    "all" {
        Format-Code
        Build-Dev
        Lint-All
        Test-All
    }
    default {
        Write-Host "Available commands:" -ForegroundColor Yellow
        Write-Host "  build-dev      - Build development version"
        Write-Host "  build-prod     - Build production version"
        Write-Host "  build-profiling- Build profiling version"
        Write-Host "  build-wheel    - Build wheel package"
        Write-Host "  test-python  - Run Python tests"
        Write-Host "  test-rust    - Run Rust tests"
        Write-Host "  test         - Run all tests"
        Write-Host "  lint-python  - Lint Python code"
        Write-Host "  lint-rust    - Lint Rust code"
        Write-Host "  lint         - Lint all code"
        Write-Host "  format       - Format all code"
        Write-Host "  benchmark    - Run Python benchmarks"
        Write-Host "  bench-rust   - Run Rust benchmarks"
        Write-Host "  flamegraph   - Run flamegraph profiling"
        Write-Host "  clean        - Clean build artifacts"
        Write-Host "  all          - Format, build, lint, and test"
    }
}
