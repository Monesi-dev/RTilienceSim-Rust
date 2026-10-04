#!/bin/bash
# Single-Processor Task Set Test (Minimal Stats Version)
# Lightweight version for fair performance comparison with C++
# Only tracks response times, no other statistics computed

set -e

echo "Building sp-taskset-minimal binary..."
cargo build --bin sp-taskset-minimal --release

echo ""
echo "Running single-processor task set test (minimal stats)..."
echo "Command: time ./target/release/sp-taskset-minimal test-ts-sp.txt -f 100000000"
echo ""

time ./target/release/sp-taskset-minimal ./txt/simulation_scenario/test-ts-sp.txt -f 10000000
