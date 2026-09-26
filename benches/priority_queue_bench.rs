use rtilience_sim_rust::base::priority_queue::PriorityQueue;
use rtilience_sim_rust::base::random_variable::{Uniform, RandomVariable};
use std::time::Instant;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

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



/// Run insert-extraction workload: populate queue with events,
/// then repeatedly extract and re-insert
fn run_insert_extraction_workload(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_time: i64,
    max_time: i64,
) -> Vec<u128> {
    let mut queue = PriorityQueue::new();
    let mut event_data: HashMap<u64, EventData> = HashMap::new();
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Populate queue with events using uniform distribution
    for i in 0..num_messages {
        let event_id = i as u64;
        let mut rng = Uniform::new(min_time, max_time, event_id);
        let time = rng.sample();
        queue.insert(event_id, time).unwrap();
        event_data.insert(event_id, EventData { rng });
    }

    // Run insert-extraction cycles with batched timing
    for _batch_num in 0..(num_operations / batch_size) {
        let batch_start = Instant::now();

        for _ in 0..batch_size {
            let node = queue.pop_first().unwrap();
            let delta = event_data.get_mut(&node.event_id).unwrap().rng.sample();
            let next_time = node.time + delta;
            queue.insert(node.event_id, next_time).unwrap();
        }

        let batch_duration = batch_start.elapsed().as_micros();
        latencies.push(batch_duration);
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
    let mut queue = PriorityQueue::new();
    let mut event_data: HashMap<u64, EventData> = HashMap::new();
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Pre-populate queue with (num_messages - batch_size) events
    for _ in 0..(num_messages - batch_size) {
        let event_id = next_event_id();
        let mut rng = Uniform::new(min_dist, max_dist, event_id);
        let time = rng.sample();
        queue.insert(event_id, time).unwrap();
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
            queue.insert(event_id, time).unwrap();
            event_data.insert(event_id, EventData { rng });
            batch_nodes.push((event_id, time));
        }

<<<<<<< Updated upstream
        // Extract, call doit, and reinsert to settle the batch
        let mut settled_events: Vec<Box<dyn Event>> = Vec::new();
        for event in batch_events {
            let mut popped_event = queue.pop_first().unwrap();
            let (should_delete, new_events) = popped_event.doit();
            if !should_delete {
                settled_events.push(popped_event.clone_box());
                queue.insert(popped_event).unwrap();
            }
            for new_event in new_events {
                queue.insert(new_event).unwrap();
            }
        }

        // Then benchmark removal of the settled batch
=======
        // Benchmark removal of the batch
>>>>>>> Stashed changes
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
    let operations: u64 = 5_000_000;
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

        // Insert-Extraction Workload
        println!("Running Insert-Extraction workload...");
        let ie_latencies = run_insert_extraction_workload(
            queue_size, operations, batch_size,
            min, queue_size as i64
        );
        let (ie_mean, ie_stddev) = variance(&ie_latencies);
        println!("  Mean: {:.2} µs, Stddev: {:.2}", ie_mean / batch_size as f64, ie_stddev / batch_size as f64);

        // Write Insert-Extraction results to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(results_file)
            .expect("Failed to open results file");

        let ie_latencies_str = ie_latencies.iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(",");
        writeln!(file, "Insertion-Extraction,{},{},{}", queue_size, batch_size, ie_latencies_str)
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
