use rtilience_sim_rust::base::priority_queue::PriorityQueue;
use rtilience_sim_rust::base::random_variable::{Uniform, RandomVariable};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

struct EventData {
    rng: Uniform,
}

fn next_event_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn main() {
    let num_messages: u64 = 100_000;
    let num_operations: u64 = 10_000;
    let batch_size: u64 = 10_000;

    println!("Profiling removal workload (BTree with simple nodes)...");
    println!("Messages: {}, Operations: {}, Batch size: {}", num_messages, num_operations, batch_size);

    let mut queue = PriorityQueue::new();
    let mut event_data: HashMap<u64, EventData> = HashMap::new();

    // Pre-populate queue
    println!("Pre-populating queue with {} events...", num_messages - batch_size);
    for _ in 0..(num_messages - batch_size) {
        let event_id = next_event_id();
        let mut rng = Uniform::new(1, num_messages as i64, event_id);
        let time = rng.sample();
        queue.insert(event_id, time).unwrap();
        event_data.insert(event_id, EventData { rng });
    }

    println!("Pre-population complete. Queue size: {}", queue.len());

    // Insert and remove cycles
    for batch_num in 0..(num_operations / batch_size) {
        if batch_num % 10 == 0 {
            println!("Batch {}/{}", batch_num, num_operations / batch_size);
        }

        let mut batch_nodes: Vec<(u64, i64)> = Vec::new();
        for _ in 0..batch_size {
            let event_id = next_event_id();
            let mut rng = Uniform::new(1, num_messages as i64, event_id);
            let time = rng.sample();
            queue.insert(event_id, time).unwrap();
            event_data.insert(event_id, EventData { rng });
            batch_nodes.push((event_id, time));
        }

        // Remove the batch
        for (id, time) in batch_nodes {
            queue.remove(time, id);
        }
    }

    println!("Removal profiling complete. Final queue size: {}", queue.len());
}
