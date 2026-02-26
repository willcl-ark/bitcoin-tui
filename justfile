_default:
    just --list

# Build using nix
[group('build')]
build:
    nix build .#

# Build using cargo
[group('build')]
buildc:
    cargo build

# Run using nix pkg, RPCHOST, RPCUSER and RPCPASS
[group('run')]
run:
    nix run .# -- --host $RPCHOST --rpcuser $RPCUSER --rpcpassword $RPCPASS

# Run using cargo, RPCHOST, RPCUSER and RPCPASS
[group('run')]
runc:
    cargo run -- --host $RPCHOST --rpcuser $RPCUSER --rpcpassword $RPCPASS

# Build, check and clippy
[group('check')]
check:
    cargo build
    cargo check
    cargo clippy
