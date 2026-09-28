use rtilience_sim_rust::base::priority_queue::PriorityQueue;
use rtilience_sim_rust::base::random_variable::{Uniform, RandomVariable};
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

fn main() {
    let num_messages: u64 = 100_000;
    let num_operations: u64 = 10_000;
    let batch_size: u64 = 10_000;

    println!("Profiling insert-extraction workload (BTree with simple nodes)...");
    println!("Messages: {}, Operations: {}, Batch size: {}", num_messages, num_operations, batch_size);

    let mut queue: PriorityQueue<EventType> = PriorityQueue::new();
    let mut event_data: HashMap<u64, EventData> = HashMap::new();

    // Populate queue
    for i in 0..num_messages {
        let event_id = i as u64;
        let mut rng = Uniform::new(1, num_messages as i64, event_id);
        let time = rng.sample();
        queue.insert(event_id, time, EventType::Arrival).unwrap();
        event_data.insert(event_id, EventData { rng });
    }

    println!("Population complete. Queue size: {}", queue.len());

    // Extract-reinsertion cycles
    for batch_num in 0..(num_operations / batch_size) {
        if batch_num % 10 == 0 {
            println!("Batch {}/{}", batch_num, num_operations / batch_size);
        }
        for _ in 0..batch_size {
            let node = queue.pop_first().unwrap();
            let delta = event_data.get_mut(&node.event_id).unwrap().rng.sample();
            let next_time = node.time + delta;
            queue.insert(node.event_id, next_time, node.event_type).unwrap();
        }
    }

    println!("Insert-extraction profiling complete. Final queue size: {}", queue.len());
}
