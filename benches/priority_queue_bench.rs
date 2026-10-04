use rtilience_sim_rust::base::priority_queue::{BTreeQueue, Node};
use rtilience_sim_rust::base::stats::{Uniform, RandomVariable};
use std::time::Instant;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
enum EventType {
    Arrival,
    Departure,
    ServiceComplete,
}

struct EventData {
    rng: Uniform,
}

fn next_event_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn variance(latencies: &[u128]) -> (f64, f64) {
    let n = latencies.len() as f64;
    let mean = latencies.iter().sum::<u128>() as f64 / n;
    let sec_mom = latencies.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>() / n;
    let var = sec_mom - mean * mean;
    let stddev = var.sqrt();
    (mean, stddev)
}



/// Run insertion workload: populate queue, then benchmark insertion and removal of batches
fn run_insertion_workload(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_dist: i64,
    max_dist: i64,
) -> Vec<u128> {
    let mut queue: BTreeQueue<EventType> = BTreeQueue::new();
    let mut event_data: HashMap<u64, EventData> = HashMap::new();
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Pre-populate queue with (num_messages - batch_size) events
    for _ in 0..(num_messages - batch_size) {
        let event_id = next_event_id();
        let mut rng = Uniform::new(min_dist, max_dist, event_id);
        let time = rng.sample();
        queue.insert(event_id, time, EventType::Arrival).unwrap();
        event_data.insert(event_id, EventData { rng });
    }

    // Run insertion cycles with batched timing
    for _batch_num in 0..(num_operations / batch_size) {
        let mut batch_nodes: Vec<(u64, i64, EventType)> = Vec::new();

        // Prepare a batch of nodes to insert
        for _ in 0..batch_size {
            let event_id = next_event_id();
            let mut rng = Uniform::new(min_dist, max_dist, event_id);
            let time = rng.sample();
            event_data.insert(event_id, EventData { rng });
            batch_nodes.push((event_id, time, EventType::Arrival));
        }

        // Benchmark insertion of the batch
        let batch_start = Instant::now();
        for (id, time, event_type) in &batch_nodes {
            queue.insert(*id, *time, *event_type).unwrap();
        }
        let batch_duration = batch_start.elapsed().as_micros();
        latencies.push(batch_duration);

        // Now remove batch
        for (id, time, event_type) in &batch_nodes {
            queue.remove(*time, *id);
        }

    }

    latencies
}

/// Run extraction workload: populate queue, then benchmark extraction and re-insertion
fn run_extraction_workload(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_time: i64,
    max_time: i64,
) -> Vec<u128> {
    let mut queue: BTreeQueue<EventType> = BTreeQueue::new();
    let mut event_data: HashMap<u64, EventData> = HashMap::new();
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Populate queue with events using uniform distribution
    for i in 0..num_messages {
        let event_id = next_event_id();
        let mut rng = Uniform::new(min_time, max_time, event_id);
        let time = rng.sample();
        queue.insert(event_id, time, EventType::Arrival).unwrap();
        event_data.insert(event_id, EventData { rng });
    }

    // Run extraction cycles with batched timing
    for _batch_num in 0..(num_operations / batch_size) {
        let batch_start = Instant::now();

        let mut extracted: Vec<Node<EventType>> = Vec::new();
        for _ in 0..batch_size {
            let node = queue.pop_first().unwrap();
        }
        let batch_duration = batch_start.elapsed().as_micros();
        latencies.push(batch_duration);

        // Re-insert extracted elements
        for _ in 0..batch_size {
            let event_id = next_event_id();
            let mut rng = Uniform::new(min_time, max_time, event_id);
            let time = rng.sample();
            queue.insert(event_id, time, EventType::Arrival).unwrap();
            event_data.insert(event_id, EventData { rng });
        }

    }

    latencies
}

/// Run removal workload: pre-populate queue with uniform distribution, then
/// repeatedly insert a batch of events and benchmark their removal
fn run_removal_workload(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_dist: i64,
    max_dist: i64,
) -> Vec<u128> {
    let mut queue: BTreeQueue<EventType> = BTreeQueue::new();
    let mut event_data: HashMap<u64, EventData> = HashMap::new();
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Pre-populate queue with (num_messages - batch_size) events
    for _ in 0..(num_messages - batch_size) {
        let event_id = next_event_id();
        let mut rng = Uniform::new(min_dist, max_dist, event_id);
        let time = rng.sample();
        queue.insert(event_id, time, EventType::Arrival).unwrap();
        event_data.insert(event_id, EventData { rng });
    }

    // Run insertion then removal cycles with batched timing
    for _batch_num in 0..(num_operations / batch_size) {
        let mut batch_nodes: Vec<(u64, i64)> = Vec::new();

        // Insert a batch of events
        for _ in 0..batch_size {
            let event_id = next_event_id();
            let mut rng = Uniform::new(min_dist, max_dist, event_id);
            let time = rng.sample();
            queue.insert(event_id, time, EventType::Arrival).unwrap();
            event_data.insert(event_id, EventData { rng });
            batch_nodes.push((event_id, time));
        }

        // Benchmark removal of the batch
        let batch_start = Instant::now();
        for (id, time) in batch_nodes {
            queue.remove(time, id);
        }
        let batch_duration = batch_start.elapsed().as_micros();
        latencies.push(batch_duration);
    }

    latencies
}

fn main() {
    use std::fs::OpenOptions;
    use std::io::Write;

    println!("Priority Queue Benchmark");
    let operations: u64 = 1_000_000;
    let min: i64 = 1;
    let results_file = "benchmark_results.txt";

    // Queue sizes to test
    let queue_sizes = vec![
        1_000,
        2_500,
        5_000,
        7_500,
        10_000,
        25_000,
        50_000,
        75_000,
        100_000,
        250_000,
        500_000,
        750_000,
        1_000_000,
        2_500_000,
        5_000_000,
        7_500_000,
        10_000_000,
        // 25_000_000,
        //50_000_000,
    ];

    for queue_size in queue_sizes {
        let batch_size = (queue_size / 100).max(1);

        println!("\n{}", "=".repeat(75));
        println!("Queue size: {}, Batch size: {}, Operations: {}", queue_size, batch_size, operations);
        println!("{}", "=".repeat(75));

        // Insertion Workload
        println!("Running Insertion workload...");
        let insertion_latencies = run_insertion_workload(
            queue_size, operations, batch_size,
            min, queue_size as i64
        );
        let (insertion_mean, insertion_stddev) = variance(&insertion_latencies);
        println!("  Mean: {:.2} µs, Stddev: {:.2}", insertion_mean / batch_size as f64, insertion_stddev / batch_size as f64);

        // Write Insertion results to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(results_file)
            .expect("Failed to open results file");

        let insertion_latencies_str = insertion_latencies.iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(",");
        writeln!(file, "Insertion,{},{},{}", queue_size, batch_size, insertion_latencies_str)
            .expect("Failed to write to results file");

        // Extraction Workload
        println!("Running Extraction workload...");
        let extraction_latencies = run_extraction_workload(
            queue_size, operations, batch_size,
            min, queue_size as i64
        );
        let (extraction_mean, extraction_stddev) = variance(&extraction_latencies);
        println!("  Mean: {:.2} µs, Stddev: {:.2}", extraction_mean / batch_size as f64, extraction_stddev / batch_size as f64);

        // Write Extraction results to file
        let extraction_latencies_str = extraction_latencies.iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(",");
        writeln!(file, "Extraction,{},{},{}", queue_size, batch_size, extraction_latencies_str)
            .expect("Failed to write to results file");

        // Removal Workload
        println!("Running Removal workload...");
        let removal_latencies = run_removal_workload(
            queue_size, operations, batch_size,
            min, queue_size as i64
        );
        let (removal_mean, removal_stddev) = variance(&removal_latencies);
        println!("  Mean: {:.2} µs, Stddev: {:.2}", removal_mean / batch_size as f64, removal_stddev / batch_size as f64);

        // Write Removal results to file
        let removal_latencies_str = removal_latencies.iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(",");
        writeln!(file, "Removal,{},{},{}", queue_size, batch_size, removal_latencies_str)
            .expect("Failed to write to results file");
    }

    println!("\n{}", "=".repeat(75));
    println!("Benchmark complete! Results written to {}", results_file);
    println!("{}", "=".repeat(75));
}
