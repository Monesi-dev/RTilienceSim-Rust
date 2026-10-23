#!/usr/bin/env python3
"""
Compares benchmark results of several priority-queue implementations.
Each input txt file holds the results of one implementation, in the same format
read by plot_benchmarks.py: workload,queue_size,batch_size,time1,time2,...

Produces two figures (one for the extract-insert workload, one for the removal
workload), each with one line per implementation showing the mean of
(execution_time / batch_size) over the queue sizes, with a +/- std band.

All parameters are hardcoded in the CONFIGURATION section below.
"""

import csv
from collections import defaultdict
from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt


# ============================== CONFIGURATION ==============================

# One entry per implementation: label shown in the legend -> results file.
INPUT_FILES = {
    "BTree": "csv/bench_btreemap.csv",
    "ArenaDLL": "csv/bench_arena_dll.csv",
    "DLL": "csv/bench_dll.csv",
    "BinaryHeap": "csv/bench_binary_heap.csv",
}

# Directory where the plots are saved.
OUTPUT_DIR = "plots/benchmarks"

# Workload names as written in the first column of the txt files.
EXTRACT_INSERT_WORKLOAD = "Insertion-Extraction"
REMOVAL_WORKLOAD = "Removal"

# Axis thresholds: set to None to let matplotlib choose automatically.
# Points with a queue size outside [X_MIN, X_MAX] are discarded.
X_MIN = 0
X_MAX = 1.05e5
Y_MIN = -0.04
Y_MAX = 0.5

# Use a logarithmic scale on the queue-size axis.
X_LOG_SCALE = True

# Draw the +/- std band around each line.
SHOW_STD = True

# Label of the y axis (the bench records batch times in microseconds).
Y_LABEL = "Execution Time / Batch Size (µs)"

# ===========================================================================


def read_benchmark_file(filepath):
    """
    Read benchmark results from a txt file.
    Format: workload,queue_size,batch_size,time1,time2,...

    Returns:
        dict: {workload: {queue_size: [normalized_times]}}
    """
    data = defaultdict(lambda: defaultdict(list))

    with open(filepath, 'r') as f:
        reader = csv.reader(f)
        for row in reader:
            if not row or len(row) < 4:
                continue

            workload = row[0].strip()
            queue_size = int(row[1])
            batch_size = int(row[2])
            execution_times = [float(t) for t in row[3:] if t.strip() != ""]

            # Normalize by batch size
            normalized_times = [t / batch_size for t in execution_times]
            data[workload][queue_size].extend(normalized_times)

    return data


def compute_statistics(queue_data):
    """
    Compute mean and std for each queue size of a single workload.

    Returns:
        (queue_sizes, means, stds) as sorted numpy arrays, restricted to [X_MIN, X_MAX].
    """
    queue_sizes = []
    means = []
    stds = []

    for queue_size in sorted(queue_data.keys()):
        if X_MIN is not None and queue_size < X_MIN:
            continue
        if X_MAX is not None and queue_size > X_MAX:
            continue

        times_array = np.array(queue_data[queue_size])
        queue_sizes.append(queue_size)
        means.append(np.mean(times_array))
        stds.append(np.std(times_array))

    return np.array(queue_sizes), np.array(means), np.array(stds)


def plot_workload(workload, title, all_data, output_path):
    """
    Plot one line per implementation for the given workload.
    """
    fig, ax = plt.subplots(figsize=(10, 6))

    for label, data in all_data.items():
        if workload not in data:
            print(f"  Warning: '{label}' has no '{workload}' rows, skipped")
            continue

        queue_sizes, means, stds = compute_statistics(data[workload])
        if len(queue_sizes) == 0:
            print(f"  Warning: '{label}' has no '{workload}' points inside the x range, skipped")
            continue

        line, = ax.plot(queue_sizes, means, 'o-', label=label, markersize=4)
        if SHOW_STD:
            ax.fill_between(queue_sizes, means - stds, means + stds,
                            color=line.get_color(), alpha=0.2)

    ax.set_title(title, fontsize=14, fontweight='bold')
    ax.set_xlabel('Queue Size (log scale)' if X_LOG_SCALE else 'Queue Size')
    ax.set_ylabel(Y_LABEL)
    if X_LOG_SCALE:
        ax.set_xscale('log')
    ax.set_xlim(left=X_MIN, right=X_MAX)
    ax.set_ylim(bottom=Y_MIN, top=Y_MAX)
    ax.grid(True, alpha=0.3)
    ax.legend(loc='upper left', fontsize=14, markerscale=2)

    plt.tight_layout()
    plt.savefig(output_path, dpi=150, bbox_inches='tight')
    plt.close()

    print(f"Saved plot: {output_path}")


def main():
    all_data = {}
    for label, filepath in INPUT_FILES.items():
        print(f"Reading {label}: {filepath}")
        all_data[label] = read_benchmark_file(filepath)

    output_dir = Path(OUTPUT_DIR)
    output_dir.mkdir(parents=True, exist_ok=True)

    print("\nGenerating plots...")
    plot_workload(EXTRACT_INSERT_WORKLOAD, 'Extract-Insert',
                  all_data, output_dir / 'extract_insert_comparison.png')
    plot_workload(REMOVAL_WORKLOAD, 'Removal',
                  all_data, output_dir / 'removal_comparison.png')

    print("\nDone!")


if __name__ == '__main__':
    main()
