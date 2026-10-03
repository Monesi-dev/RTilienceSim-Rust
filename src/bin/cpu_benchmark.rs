use rtilience_sim_rust::cpu::CPUSimulatorSP;
use rtilience_sim_rust::base::prelude::SimTime;
use rtilience_sim_rust::base::random_variable::{Uniform, Deterministic};

fn main() {
    println!("Starting CPU Scheduler Simulation");

    let mut simulator = CPUSimulatorSP::new()
        .with_end_time(10000)
        // === Old tasks (commented out) ===
        // .add_task_type(
        //     0 as SimTime,     // start_time
        //     100 as SimTime,   // relative_deadline
        //     1 as u32,         // priority
        //     Box::new(Uniform::new(100, 200, 0)),
        //     Box::new(Uniform::new(100, 200, 1)),
        //     true,             // store_response_times
        // )
        // .add_task_type(
        //     50 as SimTime,
        //     200 as SimTime,
        //     2 as u32,
        //     Box::new(Uniform::new(100, 200, 2)),
        //     Box::new(Uniform::new(100, 200, 3)),
        //     true,             // store_response_times
        // )
        // .add_task_type(
        //     100 as SimTime,
        //     400 as SimTime,
        //     3 as u32,
        //     Box::new(Uniform::new(100, 200, 4)),
        //     Box::new(Uniform::new(100, 200, 5)),
        //     false,            // store_response_times
        // )
        // .add_task_type(
        //     200 as SimTime,
        //     10 as SimTime,
        //     4 as u32,
        //     Box::new(Uniform::new(100, 200, 6)),
        //     Box::new(Uniform::new(100, 200, 7)),
        //     false,            // store_response_times
        // )

        // === New periodic tasks (deterministic) ===
        // Task 1: Period=100, Execution=20, Deadline=100, Priority=1
        .add_task_type(
            0 as SimTime,
            100 as SimTime,   // relative_deadline
            1 as u32,         // priority (highest)
            Box::new(Deterministic::new(20)),    // fixed execution time
            Box::new(Deterministic::new(100)),   // fixed period
            true,             // store_response_times
        )
        // Task 2: Period=200, Execution=40, Deadline=200, Priority=2
        .add_task_type(
            0 as SimTime,
            200 as SimTime,   // relative_deadline
            2 as u32,         // priority
            Box::new(Deterministic::new(40)),    // fixed execution time
            Box::new(Deterministic::new(200)),   // fixed period
            true,             // store_response_times
        )
        // Task 3: Period=300, Execution=50, Deadline=300, Priority=3
        .add_task_type(
            0 as SimTime,
            300 as SimTime,   // relative_deadline
            3 as u32,         // priority
            Box::new(Deterministic::new(50)),    // fixed execution time
            Box::new(Deterministic::new(300)),   // fixed period
            true,             // store_response_times
        )
        // Task 4: Period=400, Execution=60, Deadline=400, Priority=4
        .add_task_type(
            0 as SimTime,
            400 as SimTime,   // relative_deadline
            4 as u32,         // priority (lowest)
            Box::new(Deterministic::new(60)),    // fixed execution time
            Box::new(Deterministic::new(400)),   // fixed period
            true,             // store_response_times
        );

    println!("Running simulation...");
    simulator.simulate();

    println!("Simulation complete!");
    simulator.print_stats();
    simulator.dump_response_times("response_times.csv").expect("Failed to dump response times");
}
