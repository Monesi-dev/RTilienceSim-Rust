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

/// Run insert-extraction workload with binary heap
fn run_insert_extraction_binary_heap(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_time: i64,
    max_time: i64,
) -> Vec<u128> {
    let mut queue = PriorityQueueBinaryHeap::with_capacity((num_messages * 2) as usize);
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Populate queue
    for i in 0..num_messages {
        let event = SporadicEvent::new(
            0,
            Box::new(Uniform::new(min_time, max_time, i as u64)),
        );
        queue.insert(Box::new(event)).unwrap();
        let mut event = queue.pop_first().unwrap();
        let (should_delete, new_events) = event.doit();
        if !should_delete {
            queue.insert(event).unwrap();
        }
        for new_event in new_events {
            queue.insert(new_event).unwrap();
        }
    }

    // Run cycles
    for _batch_num in 0..(num_operations / batch_size) {
        let batch_start = Instant::now();

        for _ in 0..batch_size {
            let mut event = queue.pop_first().unwrap();
            let (should_delete, new_events) = event.doit();
            if !should_delete {
                queue.insert(event).unwrap();
            }
            for new_event in new_events {
                queue.insert(new_event).unwrap();
            }
        }

        let batch_duration = batch_start.elapsed().as_micros();
        latencies.push(batch_duration);
    }

    latencies
}

/// Run removal workload with binary heap
fn run_removal_binary_heap(
    num_messages: u64,
    num_operations: u64,
    batch_size: u64,
    min_dist: i64,
    max_dist: i64,
) -> Vec<u128> {
    let mut queue = PriorityQueueBinaryHeap::with_capacity((num_messages * 2) as usize);
    let mut latencies: Vec<u128> = Vec::with_capacity((num_operations / batch_size) as usize);

    // Pre-populate queue with events using uniform distribution
    for i in 0..(num_messages - batch_size) {
        let event = SporadicEvent::new(
            0,
            Box::new(Uniform::new(min_dist, max_dist, i as u64)),
        );
        queue.insert(Box::new(event)).unwrap();
        let mut event = queue.pop_first().unwrap();
        let (should_delete, new_events) = event.doit();
        if !should_delete {
            queue.insert(event).unwrap();
        }
        for new_event in new_events {
            queue.insert(new_event).unwrap();
        }
    }

    // Run cycles: insert a batch, then remove it
    for batch_num in 0..(num_operations / batch_size) {
        let mut batch_handles: Vec<u64> = Vec::new();

        // Insert a batch of events using uniform distribution
        for i in 0..batch_size {
            let event_idx = num_messages - batch_size + batch_num * batch_size + i;
            let event = SporadicEvent::new(
                0,
                Box::new(Uniform::new(min_dist, max_dist, event_idx as u64)),
            );
            let handle = queue.insert(Box::new(event)).unwrap();
            batch_handles.push(handle);
        }

        // Extract, call doit, and reinsert to settle the batch
        let mut settled_handles: Vec<u64> = Vec::new();
        for handle in batch_handles {
            let mut popped_event = queue.pop_first().unwrap();
            let (should_delete, new_events) = popped_event.doit();
            if !should_delete {
                queue.insert(popped_event).unwrap();
                settled_handles.push(handle);
            }
            for new_event in new_events {
                queue.insert(new_event).unwrap();
            }
        }

        // Benchmark removal of the settled batch
        let batch_start = Instant::now();
        for handle in settled_handles {
            queue.remove(handle);
        }
        let batch_duration = batch_start.elapsed().as_micros();
        latencies.push(batch_duration);
    }

    latencies
}

fn main() {
    use std::fs::OpenOptions;
    use std::io::Write;

    println!("Binary Heap Benchmark");
    let operations: u64 = 5_000_000;
    let min: i64 = 1;
    let results_file = "csv/bench_binary_heap.csv";

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
        10_000_000
    ];

    for queue_size in queue_sizes {
        let batch_size = (queue_size / 100).max(1);

        println!("\n{}", "=".repeat(80));
        println!("Queue size: {}, Batch size: {}, Operations: {}", queue_size, batch_size, operations);
        println!("{}", "=".repeat(80));

        // Insert-Extraction with Binary Heap
        println!("Running Insert-Extraction (Binary Heap)...");
        let ie_bheap = run_insert_extraction_binary_heap(
            queue_size, operations, batch_size,
            min, queue_size as i64
        );
        let (ie_mean, ie_stddev) = variance(&ie_bheap);
        println!("  Mean: {:.2} µs, Stddev: {:.2}", ie_mean / batch_size as f64, ie_stddev / batch_size as f64);

        // Write Insert-Extraction results
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(results_file)
            .expect("Failed to open results file");

        write!(file, "Insertion-Extraction,{},{}", queue_size, batch_size).expect("Failed to write");
        for latency in &ie_bheap {
            write!(file, ",{}", latency).expect("Failed to write");
        }
        writeln!(file).expect("Failed to write newline");

        // Removal with Binary Heap
        println!("Running Removal (Binary Heap)...");
        let removal_bheap = run_removal_binary_heap(
            queue_size, operations, batch_size,
            min, queue_size as i64
        );
        let (removal_mean, removal_stddev) = variance(&removal_bheap);
        println!("  Mean: {:.2} µs, Stddev: {:.2}", removal_mean / batch_size as f64, removal_stddev / batch_size as f64);

        // Write Removal results
        write!(file, "Removal,{},{}", queue_size, batch_size).expect("Failed to write");
        for latency in &removal_bheap {
            write!(file, ",{}", latency).expect("Failed to write");
        }
        writeln!(file).expect("Failed to write newline");
    }

    println!("\n{}", "=".repeat(80));
    println!("Benchmark complete! Results written to {}", results_file);
    println!("{}", "=".repeat(80));
}
