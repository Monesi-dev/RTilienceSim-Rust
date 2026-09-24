## Benchmarks

**Execution Time**: Run `XXX_bench.rs` to measure insertion, extraction, and removal performance across queue implementations. Build with `cargo bench --bench <bench_name>`. Names are in Cargo.toml under the \[\[bench\]\] section with the matching path. The results are stored in `/csv`, the plots are under `/plots` and the scripts to build the plots are under `/scripts`.

**Memory Usage**: Run `profile_{insert_extraction|removal}_XXX.rs` to profile heap allocation. Build with `cargo build --release --bin <name>`, then execute with `heaptrack <executable>` and analyze with `heaptrack --analyze <artifact>`. Names are in Cargo.toml under the \[\[bin\]\] section with the matching path. The artifact are stored in `assets/memory_profiling/heaptrack_artifacts` and the results are under `assets/memory_profiling/results`.

**Cache Performance**: Use `perf stat -e cache-references,cache-misses <executable>` to compare cache hits between implementations. The results are under `assets/cache_profiling`.

**Assembly Comparison**: I have edited the priority_queue rust files for both dll and arena_dll to let them be used by godbolt, a web-app with a disassembler, the files are under `assets\assembly` together with the assembly output of the insert function

