use rtilience_sim_rust::base::prelude::*;
use std::time::Instant;

fn variance(latencies: &[u128]) -> (f64, f64) {
    let n = latencies.len() as f64;
    let mean = latencies.iter().sum::<u128>() as f64 / n;
    let sec_mom = latencies.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>() / n;
    let var = sec_mom - mean * mean;
    let stddev = var.sqrt();
    (mean, stddev)
}



/// Run insert-extraction workload: populate queue with sporadic events,
/// then repeatedly extract, call doit(), and re-insert
fn run_insert_extraction_workload(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_time: i64,
    max_time: i64,
) -> Vec<u128> {
    let mut queue = PriorityQueue::new();
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Populate queue with sporadic events using uniform distribution
    for i in 0..num_messages {
        let event = SporadicEvent::new(
            0,
            Box::new(Uniform::new(min_time, max_time, i as u64)),
        );
        queue.insert(Box::new(event)).unwrap();
        let mut event = queue.pop_first().unwrap();
        let new_events = event.doit();
        for new_event in new_events {
            queue.insert(new_event).unwrap();
        }
    }

    // Run insert-extraction cycles with batched timing
    for _batch_num in 0..(num_operations / batch_size) {
        let batch_start = Instant::now();

        for _ in 0..batch_size {
            let mut event = queue.pop_first().unwrap();
            let new_events = event.doit();
            for new_event in new_events {
                queue.insert(new_event).unwrap();
            }
        }

        let batch_duration = batch_start.elapsed().as_micros();
        let per_op_latency = batch_duration as u128;
        latencies.push(per_op_latency);
    }

    latencies
}

/// Run removal workload: pre-populate queue, then repeatedly insert a batch of events
/// and benchmark their removal
fn run_removal_workload(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_dist: i64,
    max_dist: i64,
) -> Vec<u128> {
    let mut queue = PriorityQueue::new();
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Pre-populate queue with (num_messages - batch_size) events
    for i in 0..(num_messages - batch_size) {
        let event = Box::new(SporadicEvent::new(
            i as SimTime,
            Box::new(Uniform::new(min_dist, max_dist, i as u64)),
        ));
        queue.insert(event).unwrap();
    }

    // Run insertion then removal cycles with batched timing
    for batch_num in 0..(num_operations / batch_size) {
        // First, insert a batch of events and save them for removal
        let mut batch_events: Vec<Box<dyn Event>> = Vec::new();
        for i in 0..batch_size {
            let event_idx = num_messages - batch_size + batch_num * batch_size + i;
            let event = Box::new(SporadicEvent::new(
                event_idx as SimTime,
                Box::new(Uniform::new(min_dist, max_dist, event_idx as u64)),
            ));
            batch_events.push(event.clone());
            queue.insert(event).unwrap();
        }

        // Then benchmark removal of the batch
        let batch_start = Instant::now();
        for event in batch_events {
            queue.remove(event);
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
        25_000_000,
        50_000_000,
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
