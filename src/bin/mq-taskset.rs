/// Multi-Processor Multi-Queue Task Set Simulator
///
/// Like mp-taskset, but each CPU has its own local ready queue and tasks are
/// statically pinned to the least-loaded CPU at registration time, instead of
/// competing on one shared global ready queue.
///
/// All parameters are hard-coded; only the task set is read from an external file.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

use rtilience_sim_rust::cpu::CPUSimulatorMQ;
use rtilience_sim_rust::cpu::StaticPriority;
use rtilience_sim_rust::cpu::mq::state::{EventType, EventPriorityPolicy};
use rtilience_sim_rust::base::stats::Deterministic;
use rtilience_sim_rust::base::prelude::{SimTime, BTreeQueue};

const TASKSET_FILE: &str = "txt/simulation_scenario/ts_N=5_U=2_1.txt";
const NUM_CPUS: usize = 4;
const SIMULATION_TIME: SimTime = 100_000_000;

struct Task {
    execution_time: SimTime,
    deadline: SimTime,
    period: SimTime,
}

fn read_taskset(filename: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let file = File::open(filename).expect("Could not open task set file");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.expect("Could not read line");
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            panic!("Warning: skipping invalid line: {}", line);
        }
        let execution_time: SimTime = parts[0].parse().expect("Could not parse execution_time");
        let deadline: SimTime = parts[1].parse().expect("Could not parse deadline");
        let period: SimTime = parts[2].parse().expect("Could not parse period");
        tasks.push(Task { execution_time, deadline, period });
    }

    tasks
}

fn print_taskset(tasks: &[Task]) {
    println!("Task Set ({} tasks):", tasks.len());
    for (i, task) in tasks.iter().enumerate() {
        println!("  Task {}: C={}, D={}, T={}", i, task.execution_time, task.deadline, task.period);
    }
}

fn main() {
    let tasks = read_taskset(TASKSET_FILE);
    if tasks.is_empty() {
        eprintln!("Error: No tasks read from file");
        process::exit(1);
    }
    print_taskset(&tasks);

    println!("Simulation Time: {}", SIMULATION_TIME);
    println!("Number of CPUs: {}", NUM_CPUS);

    let mut simulator: CPUSimulatorMQ<BTreeQueue<EventType, EventPriorityPolicy>, StaticPriority> =
        CPUSimulatorMQ::new(NUM_CPUS).with_end_time(SIMULATION_TIME);

    // Priority is assigned so the LAST task in the file gets the highest priority
    // (lowest numeric value), matching the C++ reference implementation's convention.
    for (idx, task) in tasks.iter().enumerate() {
        simulator = simulator.add_task_type(
            0,
            task.deadline,
            (tasks.len() - idx) as u64,
            Box::new(Deterministic::new(task.execution_time)),
            Box::new(Deterministic::new(task.period)),
            true,
        );
    }

    let start = std::time::Instant::now();
    simulator.simulate();
    let elapsed = start.elapsed();
    println!("Simulation completed in {:.3}s", elapsed.as_secs_f64());
    println!();

    simulator.print_stats();
}
