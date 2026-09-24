use rtilience_sim_rust::base::prelude::*;
use std::io::{self, Write};

fn main() {
    let num_messages: u64 = 100_000;
    let num_operations: u64 = 10_000;
    let batch_size: u64 = 10_000;

    println!("Profiling insert-extraction workload (memory only)...");
    println!("Messages: {}, Operations: {}, Batch size: {}", num_messages, num_operations, batch_size);

    let mut queue = PriorityQueue::new();

    // Populate queue
    for i in 0..num_messages {
        let event = SporadicEvent::new(
            0,
            Box::new(Uniform::new(1, num_messages as i64, i as u64)),
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

    println!("Population complete. Queue size: {}", queue.len());

    // Extract-reinsertion cycles
    for batch_num in 0..(num_operations / batch_size) {
        if batch_num % 10 == 0 {
            println!("Batch {}/{}", batch_num, num_operations / batch_size);
        }
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
    }

    println!("Insert-extraction profiling complete. Final queue size: {}", queue.len());
}
