_default:
    just --list

# Build using nix
[group('dev')]
build:
    nix build .#

# Build using cargo
[group('dev')]
buildc:
    cargo build

# Run using nix pkg, RPCHOST, RPCUSER and RPCPASS
[group('run')]
run:
    nix run .# -- --host $RPCHOST --rpcuser $RPCUSER --rpcpassword $RPCPASS --zmqhost $RPCHOST --zmqport 29000

# Debug using nix run
[group('dev')]
debug:
    nix run .# -- --host $RPCHOST --rpcuser $RPCUSER --rpcpassword $RPCPASS --zmqhost $RPCHOST --zmqport 29000 --debug 2> debug.log

# Run using cargo, RPCHOST, RPCUSER and RPCPASS
[group('run')]
runc:
    cargo run -- --host $RPCHOST --rpcuser $RPCUSER --rpcpassword $RPCPASS --zmqhost $RPCHOST --zmqport 29000 --debug 2> debug.log

# Build, check and clippy
[group('lint')]
check:
    cargo build
    cargo check
    cargo clippy
