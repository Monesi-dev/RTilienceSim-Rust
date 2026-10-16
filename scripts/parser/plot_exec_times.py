#!/usr/bin/env python3
"""Plots mean +- std execution time vs N, one grouped bar chart per U, comparing
the priority-queue implementations listed in FILES."""

import csv
import statistics
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

# --- Hardcoded inputs -------------------------------------------------------

FILES = [
    #("txt/exec_times/sp_btree.txt", "BTreeQueue"),
    #("txt/exec_times/sp_arenadll.txt", "ArenaQueue"),
    #("txt/exec_times/sp_binaryheap.txt", "BinaryHeapQueue"),
    #("txt/exec_times/sp_vec.txt", "VecQueue"),
    ("txt/exec_times/cpp_sp.txt", "C++"),
    ("txt/exec_times/sp_arenadll.txt", "Rust"),
]

N_LIST = [2, 4, 6, 8, 10, 25, 50, 100]
U_LIST = [0.3, 0.5, 0.7, 0.9]
ID_LIST = list(range(1, 11))

# Fixed categorical hue order (never cycled), one slot per file in FILES
COLORS = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100"]

# --- Read data ---------------------------------------------------------------

# stats[U][plotname][N] = (mean, std)
stats: dict[float, dict[str, dict[int, tuple[float, float]]]] = {
    U: {plotname: {} for _, plotname in FILES} for U in U_LIST
}

for filename, plotname in FILES:
    # times_by_nu[(N, U)] = list of times across ids
    times_by_nu: dict[tuple[int, float], list[float]] = {}

    with open(filename, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            n = int(row["N"])
            u = float(row["U"])
            event_id = int(row["id"])
            time = float(row["time_seconds"])

            if n not in N_LIST or u not in U_LIST or event_id not in ID_LIST:
                continue

            times_by_nu.setdefault((n, u), []).append(time)

    for u in U_LIST:
        for n in N_LIST:
            times = times_by_nu.get((n, u), [])
            mean_time = statistics.mean(times)
            std_time = statistics.stdev(times) if len(times) > 1 else 0.0
            stats[u][plotname][n] = (mean_time, std_time)

# --- Plot ---------------------------------------------------------------------

# Two N ranges per U, plotted as separate figures so the large-N bars don't
# squash the small-N ones under a shared linear y-axis.
N_RANGES = [
    ("N_2-10", [n for n in N_LIST if 2 <= n <= 10]),
    ("N_10-100", [n for n in N_LIST if 10 <= n <= 100]),
]

n_files = len(FILES)
group_width = 0.8
bar_width = group_width / n_files

for u in U_LIST:
    for suffix, n_range in N_RANGES:
        fig, ax = plt.subplots(figsize=(10, 6))
        x = np.arange(len(n_range))

        for i, (_, plotname) in enumerate(FILES):
            means = [stats[u][plotname][n][0] for n in n_range]
            stds = [stats[u][plotname][n][1] for n in n_range]
            offset = (i - (n_files - 1) / 2) * bar_width
            ax.bar(
                x + offset,
                means,
                bar_width,
                yerr=stds,
                capsize=3,
                label=plotname,
                color=COLORS[i % len(COLORS)],
            )

        ax.set_xlabel("N (number of tasks)")
        ax.set_ylabel("Execution time (s)")
        ax.set_title(f"Execution time vs N (U = {u}, N in [{n_range[0]}, {n_range[-1]}])")
        ax.set_xticks(x)
        ax.set_xticklabels(n_range)
        ax.legend()
        ax.grid(axis="y", linestyle="--", alpha=0.3)
        fig.tight_layout()

        # out_path = Path(f"plots/exec_times/sp_U_{u}_{suffix}.png")
        out_path = Path(f"plots/exec_times/sp_cmp_U_{u}_{suffix}.png")
        fig.savefig(out_path, dpi=150)
        plt.close(fig)
        print(f"Saved {out_path}")
