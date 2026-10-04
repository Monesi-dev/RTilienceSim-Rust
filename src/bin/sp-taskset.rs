/// Single-Processor Task Set Simulator
///
/// Equivalent to Tesina/cpu/test/sp-taskset.cc
///
/// Usage: sp-taskset <taskset_file> [-f finishing_time] [-l num_hyperperiods]
///
/// Task set file format:
/// execution_time deadline period
/// execution_time deadline period
/// ...

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

use rtilience_sim_rust::cpu::CPUSimulatorSP;
use rtilience_sim_rust::cpu::sp::state::EventType;
use rtilience_sim_rust::base::stats::Deterministic;
use rtilience_sim_rust::base::prelude::{SimTime, BTreeQueue};

// Holds info on task descripiton in the file <exec_time, deadline, period>
struct Task {
    execution_time: SimTime,
    deadline: SimTime,
    period: SimTime,
}

// Holds the arguments that should be passed to executable:
// > Name of the text file with the tasks
// > Simulation time
// > Number of hyperperiods (used if finishing time is not passed, you compute the
// lcm of periods and run for X times the lcm)
struct Args {
    taskset_file: String,
    finishing_time: Option<SimTime>,
    num_hyperperiods: u32,
}

// Greatest Common Divisot, it is used to compute lcm which is used for hyperperiod
fn gcd(mut a: SimTime, mut b: SimTime) -> SimTime {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

// Largest Common Multiplier, it is used to compute hyperperiod among tasks
fn lcm(a: SimTime, b: SimTime) -> SimTime {
    (a / gcd(a, b)) * b
}

// Parses arguments and return args struct
fn parse_args() -> Args {

    // Arguments are parsed into a vector of strings
    let args: Vec<String> = env::args().collect();

    // Checks if there are at leat two arguments
    if args.len() < 2 {
        eprintln!("Usage: {} <taskset_file> [-f finishing_time] [-l num_hyperperiods]", args[0]);
        process::exit(1);
    }

    // Initializes struct holding argument with filename and a default number of
    // hyperperiods
    let mut result = Args {
        taskset_file: args[1].clone(),
        finishing_time: None,
        num_hyperperiods: 2,
    };

    // It iterates over remaining arguments. If it finds a known flag (i.e. -f, -l)
    // it tries to read its value in the following argument (if not available it exits)
    // and updates the args struct. If it finds an unrecognizable arg it exits
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-f" => {
                if i + 1 < args.len() {
                    result.finishing_time = Some(args[i + 1].parse().expect("Invalid finishing_time"));
                    i += 2;
                } else {
                    eprintln!("Error: -f requires an argument");
                    process::exit(1);
                }
            }
            "-l" => {
                if i + 1 < args.len() {
                    result.num_hyperperiods = args[i + 1].parse().expect("Invalid num_hyperperiods");
                    i += 2;
                } else {
                    eprintln!("Error: -l requires an argument");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                process::exit(1);
            }
        }
    }

    result
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

// Given the slice of task vector returns the hyperperiod (lcm of task periods)
fn compute_hyperperiod(tasks: &[Task]) -> SimTime {

    // If there are no tasks panic, it should not happen
    if tasks.is_empty() {
        panic!("No tasks available");
    }

    // You have a current hyperperiod which is updated as lcm between itself and next
    // task period
    let mut hyper = tasks[0].period;
    for task in &tasks[1..] {
        hyper = lcm(hyper, task.period);
    }
    hyper
}

// Printing tasks inside a slice of tasks
fn print_taskset(tasks: &[Task]) {
    println!("Task Set ({} tasks):", tasks.len());
    for (i, task) in tasks.iter().enumerate() {
        println!("  Task {}: C={}, D={}, T={}", i, task.execution_time, task.deadline, task.period);
    }
}

fn main() {

    // Parses arguments
    let args = parse_args();

    // Read task set and prints it
    let tasks = read_taskset(&args.taskset_file);
    if tasks.is_empty() {
        eprintln!("Error: No tasks read from file");
        process::exit(1);
    }
    print_taskset(&tasks);

    // If finish time is specified use that otherwise use hyperperiod * num_hyperperiods
    // as max simulation time
    let hyperperiod = compute_hyperperiod(&tasks);
    let finishing_time = args.finishing_time.unwrap_or(
        hyperperiod * args.num_hyperperiods as SimTime
    );
    println!("Simulation Time: {}", finishing_time);
    if let Some(_) = args.finishing_time {
        println!("Specified as CLI arguments");
    } else {
        println!("Computed as Hyperperiod: {} by Number of Hyperperiods: {}", hyperperiod, args.num_hyperperiods);
    }

    // Build simulator with BTreeQueue (BTreeMap-based) EventQueue implementation
    let mut simulator: CPUSimulatorSP<BTreeQueue<EventType>> = CPUSimulatorSP::new().with_end_time(finishing_time);

    // Add tasks with deterministic execution times
    // Priority is assigned based on task order (task index + 1)
    for (idx, task) in tasks.iter().enumerate() {
        simulator = simulator.add_task_type(
            0,                                                  // start_time
            task.deadline,                                      // relative_deadline
            (idx + 1) as u32,                                   // priority (1-indexed)
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
