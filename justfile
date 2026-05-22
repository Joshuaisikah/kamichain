# KamiChain — common dev commands
# Install just: cargo install just

# Run the full test suite
test:
    cargo test --workspace

# Run tests for one crate
test-core:
    cargo test -p kamichain-core

test-wallet:
    cargo test -p kamichain-wallet

test-node:
    cargo test -p kamichain-node

# Run only end-to-end tests
test-e2e:
    cargo test -p kamichain-node --test e2e_tests

# Run clippy across the workspace
lint:
    cargo clippy --workspace -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Apply formatting
fmt:
    cargo fmt --all

# Run all benchmarks
bench:
    cargo bench -p kamichain-core

# Build everything in release mode
build:
    cargo build --workspace --release

# Start a node on the default port with difficulty 3
node:
    cargo run --bin kamichain-node -- --bind 0.0.0.0:8332 --difficulty 3

# Start a second node that connects to the first
node2:
    cargo run --bin kamichain-node -- --bind 0.0.0.0:8333 --difficulty 3 --peer 127.0.0.1:8332

# Generate a new wallet and print the address
wallet-new:
    cargo run --bin kami -- wallet new

# Check the balance of an address (pass ADDRESS=xxx)
balance ADDRESS:
    cargo run --bin kami -- wallet balance {{ADDRESS}}

# Mine one block (pass ADDRESS=xxx)
mine ADDRESS:
    cargo run --bin kami -- mine --address {{ADDRESS}}

# Show chain info from a running node
info:
    cargo run --bin kami -- chain info

# Run all checks (CI equivalent)
ci: fmt-check lint test
