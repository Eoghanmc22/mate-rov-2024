#! /usr/bin/env nix-shell
#! nix-shell -i bash shell.nix

cd $(dirname $0)
kitty bash -c "cargo run --release --bin surface"
