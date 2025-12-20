#!/bin/sh

set -e
clear

BLUE='\033[34;3m'
GREEN='\033[32m'
RESET='\033[0m'

printf '[1/3] Action `%bLint%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo clippy%b`...\n' "$GREEN" "$RESET"
cargo clippy

printf '[2/3] Action `%bBuild%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo build --release%b`...\n' "$GREEN" "$RESET"
cargo build --release

printf '[3/3] Action `%bTest%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo test%b`...\n' "$GREEN" "$RESET"
cargo test

