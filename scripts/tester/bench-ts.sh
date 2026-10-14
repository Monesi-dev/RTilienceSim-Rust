#!/bin/bash
# Benchmark task sets with different N, U, and id values
# Runs simulator on all combinations and reports execution times

set -e

# Configuration
FINISHING_TIME=100000000
BINARY="./target/release/sp-taskset"
TASKSET_DIR="./txt/simulation_scenario"

# Build once
echo "Building sp-taskset binary..."
cargo build --bin sp-taskset --release
echo ""

# Print header
echo "N,U,id,time_seconds"

# Loop over all combinations
N_VALUES=(2 4 6 8 10 25 50 100)
# N_VALUES=(2 10 100 200)
U_VALUES=(0.3 0.5 0.7 0.9)
# U_VALUES=(0.9)

for N in "${N_VALUES[@]}"; do
    for U in "${U_VALUES[@]}"; do
        for id in {1..10}; do
        #for id in {1..1}; do
            taskset="$TASKSET_DIR/ts_N=${N}_U=${U}_${id}.txt"

            # Run and capture execution time using /usr/bin/time
            elapsed=$( /usr/bin/time -f "%e" $BINARY "$taskset" -f "$FINISHING_TIME" 2>&1 1>/dev/null )

            echo "$N,$U,$id,$elapsed"
        done
    done
done
