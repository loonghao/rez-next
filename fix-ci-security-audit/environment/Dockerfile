FROM rust:1.85-bookworm

# Install cargo-audit and git
RUN cargo install cargo-audit \
    && apt-get update \
    && apt-get install -y git jq \
    && rm -rf /var/lib/apt/lists/*

# Clone the repository at the known-vulnerable state
RUN git clone https://github.com/loonghao/rez-next.git /workspace \
    && cd /workspace \
    && git checkout fb588dc

WORKDIR /workspace

# Pre-build dependencies to speed up agent runs
RUN cargo fetch

# Create log directories
RUN mkdir -p /logs/verifier /logs/agent /solution /tests
