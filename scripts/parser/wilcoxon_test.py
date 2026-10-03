#!/usr/bin/env python3
"""
Performs Wilcoxon signed-rank tests comparing benchmark results from two files.
For each workload-queue_size pair, compares execution times normalized by batch size.
"""

import sys
import csv
from collections import defaultdict
from pathlib import Path

import numpy as np
from scipy.stats import wilcoxon


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


def main():
    if len(sys.argv) < 3:
        print("Usage: python wilcoxon_test.py <file1.txt> <file2.txt>")
        sys.exit(1)

    file1 = sys.argv[1]
    file2 = sys.argv[2]

    print(f"Reading benchmark files...")
    print(f"  File 1: {file1}")
    print(f"  File 2: {file2}")

    data1 = read_benchmark_file(file1)
    data2 = read_benchmark_file(file2)

    # Find common workloads and queue sizes
    workloads1 = set(data1.keys())
    workloads2 = set(data2.keys())
    common_workloads = workloads1 & workloads2

    if not common_workloads:
        print("Error: No common workloads found in both files!")
        sys.exit(1)

    print(f"\nFound {len(common_workloads)} common workload(s)")

    print("\n" + "="*80)
    print("Wilcoxon Signed-Rank Test Results (by Workload and Queue Size)")
    print("="*80)

    total_tests = 0
    significant_tests = 0

    for workload in sorted(common_workloads):
        queue_sizes1 = set(data1[workload].keys())
        queue_sizes2 = set(data2[workload].keys())
        common_queue_sizes = queue_sizes1 & queue_sizes2

        if not common_queue_sizes:
            print(f"\n{workload}: No common queue sizes")
            continue

        print(f"\n{workload}:")
        print("-" * 80)
        print(f"{'Queue Size':<12} {'N':<6} {'File1 Mean':<14} {'File2 Mean':<14} "
              f"{'Stat':<10} {'P-value':<12} {'Significance':<20}")
        print("-" * 80)

        for queue_size in sorted(common_queue_sizes):
            times1 = np.array(data1[workload][queue_size])
            times2 = np.array(data2[workload][queue_size])

            # Ensure same length for paired test
            min_len = min(len(times1), len(times2))
            times1 = times1[:min_len]
            times2 = times2[:min_len]

            # Perform Wilcoxon signed-rank test
            statistic, p_value = wilcoxon(times1, times2)

            # Compute summary statistics
            mean1 = np.mean(times1)
            mean2 = np.mean(times2)

            # Determine significance level
            if p_value < 0.001:
                significance = "***"
            elif p_value < 0.01:
                significance = "**"
            elif p_value < 0.05:
                significance = "*"
            else:
                significance = "ns"

            print(f"{queue_size:<12} {min_len:<6} {mean1:<14.6f} {mean2:<14.6f} "
                  f"{statistic:<10.1f} {p_value:<12.6f} {significance:<20}")

            total_tests += 1
            if p_value < 0.05:
                significant_tests += 1

    print("\n" + "="*80)
    print(f"Summary: {significant_tests}/{total_tests} tests showed significant differences (p < 0.05)")
    print("="*80)
    print("Done!")


if __name__ == '__main__':
    main()
