# rtilience_sim_rust

A Rust discrete-event simulator for preemptive CPU scheduling. It reimplements the CPU
schedulers of the C++ [RTilienceSim](../RTilienceSim) framework, with the aim of keeping C++ performance while having memory safety guarantees. This work is part of my master thesis at Scuola Superiore Sant'Anna.

The simulator uses the event-scheduling approach: a single priority queue holds future
events ordered by `(time, event priority, id)`, and the main loop pops the earliest one and
runs its handler until an `EndSimulation` event comes out.

## Project layout

| Path | Contents |
|---|---|
| `src/base/priority_queue/` | `EventQueue` trait and its implementations: `BTreeQueue`, `ArenaQueue` (arena-allocated doubly linked list), `BinaryHeapQueue`, `VecQueue` |
| `src/base/stats/` | Random variables for execution and interarrival times (deterministic, uniform, exponential, normal, Poisson, file-driven, ...) |
| `src/cpu/` | The scheduler simulators, one module each (see below) |
| `src/bin/` | One entry point per scheduler, reading a taskset file |
| `benches/` | Priority-queue benchmarks and memory-profiling programs |
| `scripts/parser/` | Python scripts that plot the results and run statistical tests |
| `scripts/tester/` | Shell scripts that build and run the simulators |
| `txt/simulation_scenario/` | Example tasksets |
| `csv/`, `plots/`, `assets/` | Benchmark results, plots, and profiling artifacts |

## Schedulers

| Module | Description |
|---|---|
| `sp` | Single processor, fixed priority or EDF |
| `mp` | Multiprocessor, one global ready queue |
| `mq` | Multiprocessor, one ready queue per CPU; each task is pinned to a CPU at registration (utilization-based) |
| `mq_lb` | Like `mq`, plus push/pull migration between CPUs |
| `mp_cbs` | Constant Bandwidth Servers, one global queue of servers |
| `mq_cbs` | Constant Bandwidth Servers, each server pinned to the least loaded CPU |
| `mp_hcbs` | Hard CBS, global queue: a server that runs out of budget sleeps until its deadline |
| `mq_hcbs` | Hard CBS, each server pinned to a CPU |

Every simulator is generic over the event-queue implementation, so the same scenario can
be run on each backend: for example `CPUSimulatorSP<BTreeQueue<..>, StaticPriority>`. The
`sp`, `mp`, `mq` and `mq_lb` simulators are also generic over the scheduling policy
(`StaticPriority` or `EDF`).

## Building and running

```sh
cargo build --release --bins
./target/release/sp-taskset txt/simulation_scenario/test-ts-sp.txt -f 10000000
```

Available binaries: `sp-taskset`, `sp-taskset-minimal`, `mp-taskset`, `mq-taskset`,
`mq_lb-taskset`, `mp_cbs-taskset`, `mq_cbs-taskset`.

- `sp-taskset <taskset_file> [-f finishing_time] [-l num_hyperperiods]` takes its
  arguments from the command line. Without `-f`, it runs for a number of hyperperiods
  (the least common multiple of the task periods).
- All the other binaries have their parameters (taskset file, number of CPUs,
  simulation length) as constants at the top of their source file in `src/bin/`. Edit
  those constants and rebuild to change them.

### Taskset file formats

For fixed-priority and EDF schedulers, each line is one task:

```
execution_time deadline period
```

Priorities are assigned by line order: the **last** line gets the highest priority. This
matches the C++ reference in `Tesina/cpu`.

For CBS schedulers, a line with two numbers declares a server (budget `Q`, period `T`).
The three-number lines that follow it are the tasks served by that server:

```
50 100          <- server: Q T
10 100 100      <- task of that server: execution_time deadline period
20 200 200
30 100          <- next server
15 150 150
```

## Tests

```sh
cargo test --lib
```

This runs the unit tests of the priority queues and of each scheduler. To test a single
module, pass its path, for example `cargo test --lib cpu::mq_hcbs`.

## Benchmarks

Some files in `benches/` use the old `base::legacy` module, which is no longer compiled,
so not every benchmark builds.

**Execution time**: the `XXX_bench.rs` files measure insertion, extraction and removal
performance across the queue implementations. Run one with `cargo bench --bench
<bench_name>`; the names are listed in `Cargo.toml` under `[[bench]]`. The results are
stored in `csv/`, the plots in `plots/`, and the scripts that build the plots in
`scripts/parser/`.

**Memory usage**: the `profile_{insert_extraction|removal}_XXX.rs` files profile heap
allocation. Their `[[bin]]` entries in `Cargo.toml` are commented out; uncomment one,
build it with `cargo build --release --bin <name>`, run it with `heaptrack <executable>`
and analyze the output with `heaptrack --analyze <artifact>`. The artifacts are stored in
`assets/memory_profiling/heaptrack_artifacts` and the results in
`assets/memory_profiling/results`.

**Cache performance**: `perf stat -e cache-references,cache-misses <executable>` compares
cache behaviour between implementations. The results are in `assets/cache_profiling`.

**Assembly comparison**: `assets/assembly` holds the `dll` and `arena_dll` priority-queue
sources, adapted so they compile on [Compiler Explorer](https://godbolt.org), together with
the generated assembly of their `insert` function.

## Plotting

The scripts in `scripts/parser/` are run from the project root:

- `plot_benchmarks.py`: plots one queue's benchmark results (mean and std of time per
  batch element against queue size), on linear and log scales
- `plot_compare_benchmarks.py`: plots several queue implementations on the same figure
- `plot_exec_times.py`: bar charts of simulator execution times for each queue backend
- `histogram.py`: response-time histograms from a simulator's response-time dump
- `wilcoxon_test.py`: Wilcoxon signed-rank test comparing two benchmark result files

`plot_compare_benchmarks.py` and `plot_exec_times.py` have their inputs hardcoded at the
top of the file. The other scripts take file paths as command-line arguments.
