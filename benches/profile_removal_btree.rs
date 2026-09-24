use rtilience_sim_rust::base::prelude::*;
use std::io::{self, Write};

fn main() {
    let num_messages: u64 = 100_000;
    let num_operations: u64 = 10_000;
    let batch_size: u64 = 10_000;

    println!("Profiling removal workload with BTreeMap (memory only)...");
    println!("Messages: {}, Operations: {}, Batch size: {}", num_messages, num_operations, batch_size);

    let mut queue = PriorityQueue::new();

    // Pre-populate queue with events using uniform distribution
    println!("Pre-populating queue with {} events...", num_messages - batch_size);
    for i in 0..(num_messages - batch_size) {
        if i % 10_000_000 == 0 && i > 0 {
            println!("  {} events inserted", i);
        }
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

    println!("Pre-population complete. Queue size: {}", queue.len());

    // Insert and remove cycles
    for batch_num in 0..(num_operations / batch_size) {
        if batch_num % 10 == 0 {
            println!("Batch {}/{}", batch_num, num_operations / batch_size);
        }

        let mut batch_events: Vec<Box<dyn Event>> = Vec::new();
        for i in 0..batch_size {
            let event_idx = num_messages - batch_size + batch_num * batch_size + i;
            let event = SporadicEvent::new(
                0,
                Box::new(Uniform::new(1, num_messages as i64, event_idx as u64)),
            );
            let event_box = Box::new(event);
            batch_events.push(event_box.clone_box());
            queue.insert(event_box).unwrap();
        }

        // Extract, call doit, and reinsert to settle the batch
        let mut settled_events: Vec<Box<dyn Event>> = Vec::new();
        for event in batch_events {
            let mut popped_event = queue.pop_first().unwrap();
            let (should_delete, new_events) = popped_event.doit();
            if !should_delete {
                queue.insert(popped_event).unwrap();
                settled_events.push(event);
            }
            for new_event in new_events {
                queue.insert(new_event).unwrap();
            }
        }

        for event in settled_events {
            queue.remove(event);
        }
    }

    println!("Removal profiling complete. Final queue size: {}", queue.len());
}
