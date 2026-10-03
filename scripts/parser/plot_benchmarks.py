#!/usr/bin/env python3
"""
Plots benchmark results showing execution times over queue sizes.
Computes mean and std of (execution_times / batch_size) for each workload-queue_size pair.
Creates two plots per workload: linear and log scale for queue_size.
"""

import sys
import csv
from collections import defaultdict
from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt


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

            workload = row[0]
            queue_size = int(row[1])
            batch_size = int(row[2])
            execution_times = [float(t) for t in row[3:]]

            # Normalize by batch size
            normalized_times = [t / batch_size for t in execution_times]
            data[workload][queue_size].extend(normalized_times)

    return data


def compute_statistics(data):
    """
    Compute mean and std for each workload-queue_size pair.

    Returns:
        dict: {workload: {queue_size: {'mean': float, 'std': float}}}
    """
    stats = defaultdict(dict)

    for workload, queue_data in data.items():
        for queue_size, times in queue_data.items():
            times_array = np.array(times)
            stats[workload][queue_size] = {
                'mean': np.mean(times_array),
                'std': np.std(times_array)
            }

    return stats


def plot_workload(workload, stats, output_dir):
    """
    Create two plots for a workload: linear and log scale.
    """
    queue_sizes = sorted(stats[workload].keys())
    means = [stats[workload][qs]['mean'] for qs in queue_sizes]
    stds = [stats[workload][qs]['std'] for qs in queue_sizes]

    fig, axes = plt.subplots(1, 2, figsize=(14, 5))
    fig.suptitle(f'Workload: {workload}', fontsize=14, fontweight='bold')

    # Linear scale plot
    ax = axes[0]
    ax.errorbar(queue_sizes, means, yerr=stds, fmt='o-', capsize=5, capthick=2)
    ax.set_xlabel('Queue Size')
    ax.set_ylabel('Execution Time / Batch Size')
    ax.set_title('Linear Scale')
    ax.grid(True, alpha=0.3)

    # Log scale plot
    ax = axes[1]
    ax.errorbar(queue_sizes, means, yerr=stds, fmt='o-', capsize=5, capthick=2)
    ax.set_xlabel('Queue Size (log scale)')
    ax.set_ylabel('Execution Time / Batch Size')
    ax.set_title('Log Scale')
    ax.set_xscale('log')
    ax.grid(True, alpha=0.3)

    # Save
    output_path = Path(output_dir) / f'{workload}_benchmark.png'
    plt.tight_layout()
    plt.savefig(output_path, dpi=150, bbox_inches='tight')
    plt.close()

    print(f"Saved plot: {output_path}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python plot_benchmarks.py <benchmark_file.txt> [output_dir]")
        sys.exit(1)

    benchmark_file = sys.argv[1]
    output_dir = sys.argv[2] if len(sys.argv) > 2 else Path(benchmark_file).parent

    print(f"Reading benchmark file: {benchmark_file}")
    data = read_benchmark_file(benchmark_file)

    print("Computing statistics...")
    stats = compute_statistics(data)

    # Print summary
    print(f"\nFound {len(stats)} workloads:")
    for workload in sorted(stats.keys()):
        num_queue_sizes = len(stats[workload])
        print(f"  {workload}: {num_queue_sizes} queue sizes")

    # Create plots for each workload
    print(f"\nGenerating plots in {output_dir}...")
    Path(output_dir).mkdir(parents=True, exist_ok=True)

    for workload in sorted(stats.keys()):
        plot_workload(workload, stats, output_dir)

    print("\nDone!")


if __name__ == '__main__':
    main()
