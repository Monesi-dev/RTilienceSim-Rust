/// Multi-Processor Multi-Queue CBS Task Set Simulator
///
/// Like mp_cbs-taskset, all parameters (task set file, number of CPUs, simulation
/// length) are hard-coded in the program instead of being read from CLI arguments. Only
/// the task set itself is read from an external file, in the same format.
///
/// The difference is in the scheduler: here every server is pinned to the least loaded
/// CPU when it is registered and never leaves it, so the per-CPU breakdown in the
/// statistics shows how the servers ended up partitioned.
///
/// Task set file format: a server line, then one line per task served by it, repeated
/// for every server. The two are told apart by how many numbers they hold.
///
/// max_budget period                      <- server (Q T), 2 numbers
/// execution_time deadline period         <- task of that server, 3 numbers
/// execution_time deadline period
/// max_budget period                      <- next server
/// execution_time deadline period
/// ...

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

use rtilience_sim_rust::cpu::CPUSimulatorMQCBS;
use rtilience_sim_rust::cpu::mq_cbs::state::{EventType, EventPriorityPolicy, TaskSpec};
use rtilience_sim_rust::base::stats::Deterministic;
use rtilience_sim_rust::base::prelude::{SimTime, BTreeQueue};

// Hard-coded simulation parameters
const TASKSET_FILE: &str = "txt/simulation_scenario/ts_cbs_N=3_1.txt";
const NUM_CPUS: usize = 2;
const SIMULATION_TIME: SimTime = 100_000;

// Holds info on task description in the file <exec_time, deadline, period>
struct Task {
    execution_time: SimTime,
    deadline: SimTime,
    period: SimTime,
}

// Holds a server <max_budget, period> together with the tasks it serves
struct Server {
    max_budget: SimTime,
    period: SimTime,
    tasks: Vec<Task>,
}

// What the parser expects on the next line. It starts out wanting a server, since a
// task cannot be read before the server that serves it, and from then on it accepts
// either a task of the server being read or the start of a new one.
enum ParserState {
    LookingForServer,
    LookingForTaskOrServer,
}

// Parses a server line <max_budget, period>, crashes if a field cannot be read
fn parse_server(parts: &[&str]) -> Server {
    let max_budget: SimTime = parts[0].parse().expect("Could not parse server max_budget");
    let period: SimTime = parts[1].parse().expect("Could not parse server period");

    Server {
        max_budget,
        period,
        tasks: Vec::new(),
    }
}

// Parses a task line <exec_time, deadline, period>, crashes if a field cannot be read
fn parse_task(parts: &[&str]) -> Task {
    let execution_time: SimTime = parts[0].parse().expect("Could not parse execution_time");
    let deadline: SimTime = parts[1].parse().expect("Could not parse deadline");
    let period: SimTime = parts[2].parse().expect("Could not parse period");

    Task {
        execution_time,
        deadline,
        period,
    }
}

// Reads the taskset file and returns the servers it describes, each already holding its
// own tasks. Lines are told apart by how many numbers they hold: two means a server,
// three means a task of the server being read.
fn read_taskset(filename: &str) -> Vec<Server> {

    let mut servers: Vec<Server> = Vec::new();
    let mut state = ParserState::LookingForServer;

    // Opens file
    let file = File::open(filename).expect("Could not open task set file");
    let reader = BufReader::new(file);

    // Reads line-by-line, feeding each one to the state machine
    for line in reader.lines() {

        // Gets line (if an error occurred crash), remove leading trailing spaces and
        // skip it if blank
        let line = line.expect("Could not read line");
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();

        match state {

            // Nothing has been read yet, so the only thing that can come is a server
            ParserState::LookingForServer => {
                if parts.len() != 2 {
                    eprintln!("Error: expected a server line <max_budget period>, found: {}", line);
                    process::exit(1);
                }
                servers.push(parse_server(&parts));
                state = ParserState::LookingForTaskOrServer;
            }

            // A server is being read, so a task line adds to it while a server line
            // closes it and opens the next one
            ParserState::LookingForTaskOrServer => {
                match parts.len() {
                    3 => servers.last_mut().unwrap().tasks.push(parse_task(&parts)),
                    2 => servers.push(parse_server(&parts)),
                    _ => {
                        eprintln!("Error: expected a task line <execution_time deadline period> or a server line <max_budget period>, found: {}", line);
                        process::exit(1);
                    }
                }
            }
        }
    }

    servers
}

// Printing servers and the tasks they serve
fn print_taskset(servers: &[Server]) {
    println!("Task Set ({} servers):", servers.len());
    for (server_idx, server) in servers.iter().enumerate() {
        println!("  Server {}: Q={}, T={}", server_idx, server.max_budget, server.period);
        for (task_idx, task) in server.tasks.iter().enumerate() {
            println!("    Task {}: C={}, D={}, T={}", task_idx, task.execution_time, task.deadline, task.period);
        }
    }
}

fn main() {

    // Read task set and prints it
    let servers = read_taskset(TASKSET_FILE);
    if servers.is_empty() {
        eprintln!("Error: No servers read from file");
        process::exit(1);
    }
    print_taskset(&servers);

    println!("Simulation Time: {}", SIMULATION_TIME);
    println!("Number of CPUs: {}", NUM_CPUS);

    // Build simulator with BTreeQueue (BTreeMap-based) EventQueue implementation
    let mut simulator: CPUSimulatorMQCBS<BTreeQueue<EventType, EventPriorityPolicy>> =
        CPUSimulatorMQCBS::new(NUM_CPUS).with_end_time(SIMULATION_TIME);

    // Register one server per entry in the file, with deterministic execution times.
    // There is no task priority to assign: inside a server jobs are ordered by their
    // own deadline, and the servers pinned to a cpu compete by their CBS scheduling
    // deadline. The cpu each server is pinned to is chosen here, in registration order.
    for server in servers {
        let mut task_specs = Vec::new();
        for task in server.tasks {
            task_specs.push(TaskSpec {
                start_time: 0,
                relative_deadline: task.deadline,
                execution_time_rng: Box::new(Deterministic::new(task.execution_time)),
                interarrival_time_rng: Box::new(Deterministic::new(task.period)),
                store_response_times: true,
            });
        }
        simulator = simulator.add_server(server.max_budget, server.period, task_specs);
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
