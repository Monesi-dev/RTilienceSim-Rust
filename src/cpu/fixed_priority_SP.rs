//! # Fixed-Priority Preemptive CPU Scheduler Simulator
//!
//! This modules simulates a fixed-priority scheduling on a single processor
//! The simulator uses an event-driven approach where:
//! - **Events** are extracted from a priority queue ordered by simulation time
//! - **Handlers** are called for each event type, modifying priority queue and global state
//!
//! ## Core Concepts
//!
//! ### Tasks and Jobs
//! - A **Task** is a periodic task type defined by relative deadline, priority, and RVs
//!   for execution time and interarrival time (time between successive job spawns)
//! - A **Job** is a concrete instance of a task spawned at a specific time. Each job 
//!   has a deadline, priority, execution time requirement, and remaining execution time
//!
//! ### Event Types
//! 1. **TaskTrigger**: A task spawns a new job
//!    - The new job is scheduled on CPU (if idle), preempts the running job (if higher
//!      priority), or is added to the ready queue (if lower priority than running job)
//!    - A new TaskTrigger is inserted with an offset sampled from interarrival times RV
//!
//! 2. **JobCompletion**: A job finishes execution on the CPU
//!    - The completed job is removed from the system
//!    - The next job from the ready queue is scheduled on CPU (if queue is not empty)
//!
//! 3. **EndSimulation**: Simulation terminates at the specified end time
//!
use crate::base::prelude::{SimTime, RandomVariable, Node, PriorityQueue};
use crate::debug_println;
use crate::get_next_id;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;


/// Enum with Event types used in this simulation
/// `TaskTrigger` means that a task is about to spawn a job
/// `JobCompletion` means that job being executed has ended
/// `EndSimulation` means that simulation has to stop
#[derive(Debug, Clone, Copy)]
enum EventType {
    TaskTrigger,
    JobCompletion,
    EndSimulation,
}

/// This is a Task Descriptor struct, it is used to store, in the global state, everything
/// that should be remembered about a task.
/// `relative_deadline` and `priority` are used to get deadline and priority of spawned job.
/// `execution_time_rng` is used to sample execution time of spawned job.
/// `interarrival_time_rng` is used to sample time after which a new job will be spawned.
#[derive(Debug)]
struct TaskType {
    relative_deadline: SimTime,
    priority: u32,
    execution_time_rng: Box<dyn RandomVariable>,
    interarrival_time_rng: Box<dyn RandomVariable>,
}

/// This is a JOb Descriptor struct, it is used to store, in the global state, everything
/// that should be known about a task
/// `scheduled_on_cpu` is the time at which it was scheduled on cpu if it the running
/// job otherwise it is None
/// `remaining_time` is the time from scheduled_on_cpu or, if None, from when it will be
/// scheduled remaining until completion
/// `priority` and `deadline` are the priority of job and its deadline
#[derive(Debug)]
struct JobDescriptor {
    scheduled_on_cpu: Option<SimTime>,
    remaining_time: SimTime,
    priority: u32,
    deadline: SimTime,
}

/// This struct holds the global state of the simulation (the whole simulation are simply
/// handlers called on the basis of the extracted event that modify the state)
/// `event_id_to_task_descr` is used to map the `event_id` of the message linked to a
/// specific task to its descriptor
/// `event_id_to_job_descr` is used to map the `event_id` of the messages linked to a
/// specific job to its descriptor
/// `job_in_cpu` holds the event_id of the job currently executed by the CPU, if CPU is
/// empty then it is NOne
/// `ready_queue` is a MinHeap keeping the Jobs waiting to be executed, it hold the pair
/// <priority, job_id>
struct GlobalState {
    event_id_to_task_desc: HashMap<u64, TaskType>,
    event_id_to_job_descr: HashMap<u64, JobDescriptor>,
    job_in_cpu: Option<u64>,
    ready_queue: BinaryHeap<Reverse<(u32, u64)>>,
}

/// This struct holds everything needed for the simulation, i.e. the global state and 
/// the Priority Queue (note that it is built using Generics)
pub struct CPUSimulatorSP {
    state: GlobalState,
    queue: PriorityQueue<EventType>,
}

impl CPUSimulatorSP {

    /// It creates an empty global state and empty priority queue
    pub fn new() -> Self {
        CPUSimulatorSP {
            state: GlobalState {
                event_id_to_task_desc: HashMap::new(),
                event_id_to_job_descr: HashMap::new(),
                job_in_cpu: None,
                ready_queue: BinaryHeap::new(),
            },
            queue: PriorityQueue::new(),
        }
    }

    /// Method followinf builder pattern, it is used to build up priority queue before
    /// running simulation by adding the EndSimulation event at a specified time
    pub fn with_end_time(mut self, end_time: SimTime) -> Self {
        self.queue.insert(get_next_id(), end_time, EventType::EndSimulation).unwrap();
        self
    }

    /// Method following the builder pattern used to construct priority queue before
    /// simulation. It takes the relative deadline and priority of a task then it takes
    /// the time at which it first spawns a job (`start_time`) and finally it takes the
    /// RandomVariables used to descrive distribution of execution times of spawned job
    /// (`execution_time_rng`) and of time that goes between two spawning of the job
    /// (`interarrival_time_rng`)
    pub fn add_task_type(
        mut self,
        start_time: SimTime,
        relative_deadline: SimTime,
        priority: u32,
        execution_time_rng: Box<dyn RandomVariable>,
        interarrival_time_rng: Box<dyn RandomVariable>,
    ) -> Self {

        // Insert TaskType event in priority queue and add Task Desc to global state
        let task = TaskType {
            relative_deadline,
            priority,
            execution_time_rng,
            interarrival_time_rng,
        };
        let event_id: u64 = get_next_id();
        self.queue.insert(event_id, start_time, EventType::TaskTrigger).unwrap();
        self.state.event_id_to_task_desc.insert(event_id, task);

        debug_println!("Added task type start_time={}, deadline={}, priority={}", start_time, relative_deadline, priority);
        self
    }

    /// Private helper method used to "schedule" job on cpu.
    /// It takes job_id and time and sets job_id as scheduled job, updates job descriptor
    /// and adds JobCompletion event
    fn schedule_job_on_cpu(&mut self, job_id: u64, current_time: SimTime) {

        self.state.job_in_cpu = Some(job_id);
        let job_desc = self.state.event_id_to_job_descr.get_mut(&job_id).unwrap();
        job_desc.scheduled_on_cpu = Some(current_time);
        let completion_time = current_time + job_desc.remaining_time;
        self.queue.insert(job_id, completion_time, EventType::JobCompletion).unwrap();

        debug_println!("Job-Id: {} scheduled on CPU. Completion-Time: {}", job_id, completion_time);
    }

    /// Private helper method used to preempt a running job.
    /// It takes the running_job_id and current_time, updates the job descriptor,
    /// removes the JobCompletion event, and adds the job to the ready queue.
    fn preempt_running_job(&mut self, running_job_id: u64, current_time: SimTime) {
        
        let preempted_job_desc = self.state.event_id_to_job_descr.get_mut(&running_job_id).unwrap();
        let scheduled_time = preempted_job_desc.scheduled_on_cpu.unwrap();
        let elapsed_time = current_time - scheduled_time;
        preempted_job_desc.remaining_time -= elapsed_time;
        preempted_job_desc.scheduled_on_cpu = None;
        self.queue.remove(scheduled_time + preempted_job_desc.remaining_time + elapsed_time, running_job_id);
        self.state.ready_queue.push(Reverse((preempted_job_desc.priority, running_job_id)));

        debug_println!("Job-Id: {} preempted. Remaining-Time: {}", running_job_id, preempted_job_desc.remaining_time
    );
    }

    /// Public method to run simulation.
    /// It is a loop of (ExtractEvent -> CallHandler) until EndSimulation event
    pub fn simulate(&mut self) {

        debug_println!("Simulate method invoked");

        loop {

            debug_println!("Extracting next event ...");

            // Extract next event, if no event found it should crash as it should not happen
            let node = self.queue.pop_first().unwrap();

            // Matching events to handlers
            match node.event_type {
                EventType::TaskTrigger => self.handle_task_trigger(node),
                EventType::JobCompletion => self.handle_job_completion(node),
                EventType::EndSimulation => {
                    self.handle_end_simulation(node);
                    break;
                }
            }
        }
    }

    /// Private handler for TaskTrigger events.
    /// Creates a new job from the task and schedules it or adds to ready queue.
    fn handle_task_trigger(&mut self, node: Node<EventType>) {
        let time = node.time;
        let event_id = node.event_id;
        debug_println!("TaskTrigger event extracted. Time: {}, Event-Id: {}", time, event_id);

        // Extract data from task_desc before calling other methods to avoid borrow issues
        let (priority, execution_time, deadline) = {
            let task_desc = self.state.event_id_to_task_desc.get_mut(&event_id).unwrap();
            debug_println!("TaskDescriptor Info: {:?}", task_desc);

            // Schedule next TaskTrigger event
            let delta = task_desc.interarrival_time_rng.sample();
            let new_time = time + delta;
            self.queue.insert(event_id, new_time, EventType::TaskTrigger).unwrap();
            debug_println!("Inserted TaskTrigger back in the queue. NewTime: {}, Event-Id: {}, Delta: {}", new_time, event_id, delta);

            // Extract what we need from task_desc
            let execution_time = task_desc.execution_time_rng.sample();
            let deadline = time + task_desc.relative_deadline;
            (task_desc.priority, execution_time, deadline)
        }; // task_desc reference dropped here

        // Create JobDescriptor for the new job
        let job_id = get_next_id();
        let job_desc = JobDescriptor {
            scheduled_on_cpu: None,
            remaining_time: execution_time,
            priority,
            deadline,
        };
        self.state.event_id_to_job_descr.insert(job_id, job_desc);
        debug_println!("Created JobDescriptor. Job-Id: {}, Execution-Time: {}, Deadline: {}, Priority: {}", job_id, execution_time, deadline, priority);

        // If CPU is empty schedule on CPU
        if self.state.job_in_cpu.is_none() {
            self.schedule_job_on_cpu(job_id, time);
        } 
        // Otherwise if it has higher priority preempt and schedule on cpu. If it has 
        // lower priority just add to ready queue
        else if let Some(running_job_id) = self.state.job_in_cpu {
            let running_job_priority = self.state.event_id_to_job_descr.get(&running_job_id).unwrap().priority;

            if priority < running_job_priority {
                debug_println!("Job-Id: {} preempts Job-Id: {}", job_id, running_job_id);
                self.preempt_running_job(running_job_id, time);
                self.schedule_job_on_cpu(job_id, time);
            } else {
                self.state.ready_queue.push(Reverse((priority, job_id)));
                debug_println!("Job-Id: {} added to ready queue. Priority: {}", job_id, priority);
            }
        }
    }

    /// Private handler for JobCompletion events.
    /// Removes the completed job and schedules the next one from ready queue.
    fn handle_job_completion(&mut self, node: Node<EventType>) {
        let time = node.time;
        let job_id = node.event_id;
        debug_println!("JobCompletion event extracted. Time: {}, Job-Id: {}", time, job_id);

        // Verify job is the one running on CPU
        assert_eq!(self.state.job_in_cpu, Some(job_id), "JobCompletion for job not in CPU");

        // Remove completed job
        self.state.job_in_cpu = None;
        self.state.event_id_to_job_descr.remove(&job_id);
        debug_println!("Job-Id: {} completed and removed from CPU", job_id);

        // Schedule next job from ready queue if available
        if let Some(Reverse((_, next_job_id))) = self.state.ready_queue.pop() {
            self.schedule_job_on_cpu(next_job_id, time);
        } else {
            debug_println!("Ready queue is empty. CPU is now idle.");
        }
    }

    /// Private handler for EndSimulation events.
    fn handle_end_simulation(&mut self, _node: Node<EventType>) {
        debug_println!("EndSimulation event extracted. Exiting");
    }
}