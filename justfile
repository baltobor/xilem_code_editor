# Build, test and quality gates for the whole workspace.

check:
    cargo check --workspace --all-targets --all-features

test:
    cargo test --workspace --all-features

clippy:
    cargo clippy --workspace --all-targets --all-features

example name:
    cargo run --release -p xilem_code_editor --example {{name}}
