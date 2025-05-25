.DEFAULT_GOAL := all

sources = python/rez_core tests/python

# using pip install cargo (via maturin via pip) doesn't get the tty handle
# so doesn't render color without some help
export CARGO_TERM_COLOR=$(shell (test -t 0 && echo "always") || echo "auto")

.PHONY: .uv
## Check that uv is installed
.uv:
	@uv -V || echo 'Please install uv: https://docs.astral.sh/uv/getting-started/installation/'

.PHONY: .pre-commit
## Check that pre-commit is installed
.pre-commit:
	@pre-commit -V || echo 'Please install pre-commit: https://pre-commit.com/'

.PHONY: install
install: .uv .pre-commit
	uv pip install -U wheel
	uv sync --frozen --all-extras
	uv pip install -v -e .
	pre-commit install

.PHONY: rebuild-lockfiles
## Rebuild lockfiles from scratch, updating all dependencies
rebuild-lockfiles: .uv
	uv lock --upgrade

.PHONY: install-rust-coverage
install-rust-coverage:
	cargo install rustfilt coverage-prepare
	rustup component add llvm-tools-preview

.PHONY: build-dev
build-dev:
	@rm -f python/rez_core/*.so
	uv run maturin develop --uv

.PHONY: build-prod
build-prod:
	@rm -f python/rez_core/*.so
	uv run maturin develop --uv --release

.PHONY: build-profiling
build-profiling:
	@rm -f python/rez_core/*.so
	uv run maturin develop --uv --profile profiling

.PHONY: build-coverage
build-coverage:
	@rm -f python/rez_core/*.so
	RUSTFLAGS='-C instrument-coverage' uv run maturin develop --uv --release

.PHONY: build-wheel
build-wheel:
	uv run maturin build --release

.PHONY: format
format:
	uv run ruff check --fix $(sources)
	uv run ruff format $(sources)
	cargo fmt

.PHONY: lint-python
lint-python:
	uv run ruff check $(sources)
	uv run ruff format --check $(sources)

.PHONY: lint-rust
lint-rust:
	cargo fmt --version
	cargo fmt --all -- --check
	cargo clippy --version
	cargo clippy --tests -- -D warnings

.PHONY: lint
lint: lint-python lint-rust

.PHONY: test-python
test-python:
	uv run pytest tests/python/

.PHONY: test-rust
test-rust:
	PYTHONPATH=$(shell uv run python -c "import sys; print(':'.join(sys.path))") \
	PYO3_PYTHON=$(shell uv run which python) \
	cargo test

.PHONY: test
test: test-python test-rust

.PHONY: testcov
testcov: build-coverage
	@rm -rf htmlcov
	@mkdir -p htmlcov
	uv run coverage run -m pytest tests/python/
	uv run coverage report
	uv run coverage html -d htmlcov/python
	coverage-prepare html python/rez_core/*.so

.PHONY: benchmark
benchmark:
	uv run pytest tests/python/ -m performance --benchmark-enable

.PHONY: bench-rust
bench-rust:
	PYTHONPATH=$(shell uv run python -c "import sys; print(':'.join(sys.path))") \
	PYO3_PYTHON=$(shell uv run which python) \
	cargo bench

.PHONY: flamegraph
flamegraph:
	@echo "Installing flamegraph if needed..."
	@which flamegraph || cargo install flamegraph
	@echo "Building with profiling symbols..."
	@$(MAKE) build-profiling
	@echo "Running flamegraph profiling..."
	PYTHONPATH=$(shell uv run python -c "import sys; print(':'.join(sys.path))") \
	PYO3_PYTHON=$(shell uv run which python) \
	flamegraph --output flamegraph.svg -- cargo bench --features flamegraph
	@echo "Flamegraph saved to flamegraph.svg"

.PHONY: all
all: format build-dev lint test

.PHONY: clean
clean:
	rm -rf `find . -name __pycache__`
	rm -f `find . -type f -name '*.py[co]' `
	rm -f `find . -type f -name '*~' `
	rm -f `find . -type f -name '.*~' `
	rm -rf .cache
	rm -rf htmlcov
	rm -rf .pytest_cache
	rm -rf *.egg-info
	rm -f .coverage
	rm -f .coverage.*
	rm -rf build
	rm -rf target/
	rm -rf python/rez_core/*.so
