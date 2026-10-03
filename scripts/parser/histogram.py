#!/usr/bin/env python3
"""
Script to compute and visualize histograms of response times from CSV file.
"""

import sys
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path


def parse_response_times_csv(filepath):
    """
    Parse the response times CSV file.

    Returns a dict mapping task_event_id to list of response times.
    Handles frequency format: response_time,frequency pairs.
    """
    response_times = {}
    current_task = None

    with open(filepath, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line.startswith('# Task'):
                event_id = line.split('event_id=')[1].rstrip(')')
                current_task = event_id
                response_times[current_task] = []
            elif current_task is not None and ',' in line:
                try:
                    response_time, frequency = line.split(',')
                    response_times[current_task].extend([int(response_time)] * int(frequency))
                except ValueError:
                    pass

    return response_times


def plot_histograms(response_times, output_file=None, bins=20):
    """
    Create histograms for each task's response times.

    Args:
        response_times: dict mapping task_event_id to list of response times
        output_file: optional file path to save the figure
        bins: number of histogram bins
    """
    num_tasks = len(response_times)

    # Create a grid of subplots
    cols = 2
    rows = (num_tasks + 1) // 2
    fig, axes = plt.subplots(rows, cols, figsize=(12, 4 * rows))

    # Flatten axes if needed
    if num_tasks == 1:
        axes = [axes]
    else:
        axes = axes.flatten()

    # Plot histogram for each task
    for idx, (task_id, times) in enumerate(sorted(response_times.items())):
        ax = axes[idx]

        ax.hist(times, bins=bins, edgecolor='black', alpha=0.7, color='steelblue')
        ax.set_title(f'Task {task_id} Response Times (n={len(times)})', fontsize=12, fontweight='bold')
        ax.set_xlabel('Response Time', fontsize=10)
        ax.set_ylabel('Frequency', fontsize=10)
        ax.grid(axis='y', alpha=0.3)

        # Add statistics
        mean = np.mean(times)
        median = np.median(times)
        std = np.std(times)
        min_rt = np.min(times)
        max_rt = np.max(times)

        stats_text = f'Mean: {mean:.2f}\nMedian: {median:.2f}\nStd Dev: {std:.2f}\nMin: {min_rt}\nMax: {max_rt}'
        ax.text(0.98, 0.97, stats_text, transform=ax.transAxes,
                fontsize=9, verticalalignment='top', horizontalalignment='right',
                bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))

    # Hide any unused subplots
    for idx in range(num_tasks, len(axes)):
        axes[idx].axis('off')

    plt.tight_layout()

    if output_file:
        plt.savefig(output_file, dpi=150, bbox_inches='tight')
        print(f"Histogram saved to {output_file}")
    else:
        plt.show()


def print_statistics(response_times):
    """
    Print detailed statistics for each task's response times.
    """
    print("\n" + "="*60)
    print("RESPONSE TIME STATISTICS")
    print("="*60)

    for task_id in sorted(response_times.keys()):
        times = response_times[task_id]
        print(f"\nTask {task_id}:")
        print(f"  Number of observations: {len(times)}")
        print(f"  Mean:                   {np.mean(times):.2f}")
        print(f"  Median:                 {np.median(times):.2f}")
        print(f"  Std Dev:                {np.std(times):.2f}")
        print(f"  Min:                    {np.min(times)}")
        print(f"  Max:                    {np.max(times)}")
        print(f"  25th percentile:        {np.percentile(times, 25):.2f}")
        print(f"   75th percentile:        {np.percentile(times, 75):.2f}")

    print("\n" + "="*60)


def main():
    if len(sys.argv) < 2:
        print("Usage: python histogram.py <csv_file> [output_image.png]")
        print("\nExample: python histogram.py response_times.csv histogram.png")
        sys.exit(1)

    csv_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else None

    # Check file exists
    if not Path(csv_file).exists():
        print(f"Error: File '{csv_file}' not found")
        sys.exit(1)

    # Parse CSV
    print(f"Reading response times from '{csv_file}'...")
    response_times = parse_response_times_csv(csv_file)

    if not response_times:
        print("Error: No response times found in CSV file")
        sys.exit(1)

    print(f"Found {len(response_times)} tasks with response time data")

    # Print statistics
    print_statistics(response_times)

    # Plot histograms
    print("\nGenerating histograms...")
    plot_histograms(response_times, output_file=output_file, bins=20)


if __name__ == '__main__':
    main()
