#!/bin/sh

set -e
clear

BLUE='\033[34;3m'
GREEN='\033[32m'
RESET='\033[0m'

printf '[1/4] Action `%bLint%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo clippy%b`...\n' "$GREEN" "$RESET"
cargo clippy

printf '[2/4] Action `%bFormat%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo fmt -- --config tab_spaces=2,max_width=120 */**/*.rs%b`...\n' "$GREEN" "$RESET"
cargo fmt -- --config tab_spaces=2,max_width=120 */**/*.rs

printf '[3/4] Action `%bBuild%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo build --release%b`...\n' "$GREEN" "$RESET"
cargo build --release

printf '[4/4] Action `%bTest%b`...\n' "$BLUE" "$RESET"
printf 'Executing `%bcargo test%b`...\n' "$GREEN" "$RESET"
cargo test

