use rtilience_sim_rust::cpu::CPUSimulatorSP;
use rtilience_sim_rust::base::prelude::SimTime;
use rtilience_sim_rust::base::random_variable::Uniform;

fn main() {
    println!("Starting CPU Scheduler Simulation");

    let mut simulator = CPUSimulatorSP::new()
        .with_end_time(1000)
        .add_task_type(
            0 as SimTime,     // start_time
            100 as SimTime,   // relative_deadline
            1 as u32,         // priority
            Box::new(Uniform::new(100, 200, 0)),
            Box::new(Uniform::new(100, 200, 1)),
        )
        .add_task_type(
            50 as SimTime,
            200 as SimTime,
            2 as u32,
            Box::new(Uniform::new(100, 200, 2)),
            Box::new(Uniform::new(100, 200, 3)),
        )
        .add_task_type(
            100 as SimTime,
            400 as SimTime,
            3 as u32,
            Box::new(Uniform::new(100, 200, 4)),
            Box::new(Uniform::new(100, 200, 5)),
        )
        .add_task_type(
            200 as SimTime,
            10 as SimTime,
            4 as u32,
            Box::new(Uniform::new(100, 200, 6)),
            Box::new(Uniform::new(100, 200, 7)),
        );

    println!("Running simulation...");
    simulator.simulate();

    println!("Simulation complete!");
}
