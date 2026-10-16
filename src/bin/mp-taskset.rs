/// Multi-Processor Task Set Simulator
///
/// Like sp-taskset, but all parameters (task set file, number of CPUs, simulation
/// length) are hard-coded in the program instead of being read from CLI arguments.
/// Only the task set itself is read from an external file.
///
/// Task set file format:
/// execution_time deadline period
/// execution_time deadline period
/// ...

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

use rtilience_sim_rust::cpu::CPUSimulatorMP;
use rtilience_sim_rust::cpu::StaticPriority;
use rtilience_sim_rust::cpu::mp::state::{EventType, EventPriorityPolicy};
use rtilience_sim_rust::base::stats::Deterministic;
use rtilience_sim_rust::base::prelude::{SimTime, BTreeQueue};

// Hard-coded simulation parameters
const TASKSET_FILE: &str = "txt/simulation_scenario/ts_N=5_U=2_1.txt";
const NUM_CPUS: usize = 4;
const SIMULATION_TIME: SimTime = 100_000_000;

// Holds info on task descripiton in the file <exec_time, deadline, period>
struct Task {
    execution_time: SimTime,
    deadline: SimTime,
    period: SimTime,
}

// Reads taskset file and returns vector of task struct
fn read_taskset(filename: &str) -> Vec<Task> {

    // Vector with task struct to be put in the simulation
    let mut tasks = Vec::new();

    // Opens file
    let file = File::open(filename).expect("Could not open task set file");
    let reader = BufReader::new(file);

    // Reads line-by-line, extract task and put it in vector
    for line in reader.lines() {

        // Gets line (if an error occurred crash), remove leading trailing spaces and
        // skip it if blank
        let line = line.expect("Could not read line");
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split it by whitespaces and if splits are not three or some of them cannot be
        // parsed as SimTime then crash the program
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            panic!("Warning: skipping invalid line: {}", line);
        }
        let execution_time: SimTime = parts[0].parse().expect("Could not parse execution_time");
        let deadline: SimTime = parts[1].parse().expect("Could not parse deadline");
        let period: SimTime = parts[2].parse().expect("Could not parse period");

        // Append Task to vector
        tasks.push(Task {
            execution_time,
            deadline,
            period,
        });
    }

    tasks
}

// Printing tasks inside a slice of tasks
fn print_taskset(tasks: &[Task]) {
    println!("Task Set ({} tasks):", tasks.len());
    for (i, task) in tasks.iter().enumerate() {
        println!("  Task {}: C={}, D={}, T={}", i, task.execution_time, task.deadline, task.period);
    }
}

fn main() {

    // Read task set and prints it
    let tasks = read_taskset(TASKSET_FILE);
    if tasks.is_empty() {
        eprintln!("Error: No tasks read from file");
        process::exit(1);
    }
    print_taskset(&tasks);

    println!("Simulation Time: {}", SIMULATION_TIME);
    println!("Number of CPUs: {}", NUM_CPUS);

    // Build simulator with BTreeQueue (BTreeMap-based) EventQueue implementation
    let mut simulator: CPUSimulatorMP<BTreeQueue<EventType, EventPriorityPolicy>, StaticPriority> =
        CPUSimulatorMP::new(NUM_CPUS).with_end_time(SIMULATION_TIME);

    // Add tasks with deterministic execution times
    // Priority is assigned so the LAST task in the file gets the highest priority
    // (lowest numeric value), matching the C++ reference implementation's convention.
    for (idx, task) in tasks.iter().enumerate() {
        simulator = simulator.add_task_type(
            0,                                                  // start_time
            task.deadline,                                      // relative_deadline
            (tasks.len() - idx) as u64,                         // priority (last task = 1 = highest)
            Box::new(Deterministic::new(task.execution_time)),  // execution_time
            Box::new(Deterministic::new(task.period)),          // period
            true,                                              // store_response_times
        );
    }

    // Run simulation and records real execution time
    let start = std::time::Instant::now();
    simulator.simulate();
    let elapsed = start.elapsed();
    println!("Simulation completed in {:.3}s", elapsed.as_secs_f64());
    println!();

    // Print statistics
    simulator.print_stats();

    // Dump response times to CSV
    let csv_file = format!("dump.csv");
    match simulator.dump_response_times(&csv_file) {
        Ok(_) => println!("Response times written to {}", csv_file),
        Err(e) => eprintln!("Error writing response times: {}", e),
    }
}
