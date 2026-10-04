#!/bin/bash
# Single-Processor Task Set Test
# Equivalent to Tesina/cpu/test/test-sp.sh

set -e

echo "Building sp-taskset binary..."
cargo build --bin sp-taskset --release

echo ""
echo "Running single-processor task set test..."
echo "Command: time ./target/release/sp-taskset test-ts-sp.txt -f 2000000000" 
echo ""

./target/release/sp-taskset ./txt/simulation_scenario/test-ts-sp.txt -f 10000000 
