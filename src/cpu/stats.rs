//! # Statistics Module
//!
//! This module implements the computation of statistics for fixed-priority SP scheduling
//!
//! The computed statistics are: 
//! - ratio of missed deadlines
//! - per-task tardiness (where tardiness is max(0, end_time - deadline)
//! - per-task response time
//! - per-task distibution
//! - cpu usage
//!


use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use crate::base::prelude::SimTime;

/// Struct holding the information needed for each job to compute statistics
/// `arrival_time` holds the time at which job was spawned, this value is not stored in 
/// the global state of the simulation but is needed when computing response time
/// `task_event_id` holds the id of the task that spawned the job, this link is not 
/// stored in the global state of the simulation but is needed to update per-task stats
pub struct JobStatDescriptor {
    pub arrival_time: SimTime,
    pub task_event_id: u64,
}

/// Struct holding the information needed for each task to compute statistics
/// `response_times` is a Vec with all the response times (only if store_response_times is true)
/// `num_jobs` is the number of jobs, used when computing "rolling" averages
/// `avg_response_time` and `avg_tardiness` are self-explanatory
/// `store_response_times` controls whether individual response times are stored in the Vec
pub struct TaskStatDescriptor {
    pub response_times: Vec<SimTime>,
    pub num_jobs: u32,
    pub avg_response_time: SimTime,
    pub avg_tardiness: SimTime,
    store_response_times: bool,
}

impl TaskStatDescriptor {

    /// Initializes task statistics with the given store_response_times flag
    fn new(store_response_times: bool) -> Self {
        TaskStatDescriptor {
            response_times: Vec::new(),
            num_jobs: 0,
            avg_response_time: 0,
            avg_tardiness: 0,
            store_response_times,
        }
    }

    /// Add a job's response time and tardiness. Only stores response time if store_response_times is true.
    fn add_job(&mut self, response_time: SimTime, tardiness: SimTime) {
        if self.store_response_times {
            self.response_times.push(response_time);
        }
        self.num_jobs += 1;

        // Update average response time
        self.avg_response_time = (self.avg_response_time * (self.num_jobs - 1) as SimTime + response_time) / self.num_jobs as SimTime;

        // Update average tardiness (simple incremental average)
        self.avg_tardiness = (self.avg_tardiness * (self.num_jobs - 1) as SimTime + tardiness) / self.num_jobs as SimTime;
    }
}

/// Main statistics struct holding all information needed to compute statistics
/// `jobs_done` and `jobs_with_missed_deadlines` are used to compute ratio of not met deadlines
/// `event_id_to_X` are hash maps mapping id of the event to the task or job
/// `cpu_time_used` accumulates total time CPU spent executing jobs
/// `last_time_used` records the time when CPU transitioned from idle to busy
/// `simulation_end_time` stores the end time of simulation for CPU usage computation
pub struct Stats {
    pub jobs_done: u32,
    pub jobs_with_missed_deadlines: u32,
    pub event_id_to_job_stat_descr: HashMap<u64, JobStatDescriptor>,
    pub event_id_to_task_stat_descr: HashMap<u64, TaskStatDescriptor>,
    cpu_time_used: SimTime,
    last_time_used: Option<SimTime>,
    simulation_end_time: Option<SimTime>,
}

impl Stats {
    /// Create a new Stats struct
    pub fn new() -> Self {
        Stats {
            jobs_done: 0,
            jobs_with_missed_deadlines: 0,
            event_id_to_job_stat_descr: HashMap::new(),
            event_id_to_task_stat_descr: HashMap::new(),
            cpu_time_used: 0,
            last_time_used: None,
            simulation_end_time: None,
        }
    }

    /// Populates event id -> task descr hash map with an empty (stats set to 0) task
    /// descr when a task is registered. Takes a flag to control whether response times are stored.
    pub fn register_task(&mut self, task_event_id: u64, store_response_times: bool) {
        self.event_id_to_task_stat_descr.insert(
            task_event_id,
            TaskStatDescriptor::new(store_response_times)
        );
    }

    /// Populates job event id -> job descr hash map with a job descr (holding job spawn
    /// time and id of the task) when a task spawns a job
    pub fn register_job(&mut self, job_event_id: u64, arrival_time: SimTime, task_event_id: u64) {
        self.event_id_to_job_stat_descr.insert(
            job_event_id,
            JobStatDescriptor {
                arrival_time,
                task_event_id,
            },
        );
    }

    /// When a job is completed this method is called. It updates the global statistics
    /// (i.e. missed deadlines) and per-task statistics
    pub fn record_job_completion(
        &mut self,
        job_event_id: u64,
        completion_time: SimTime,
        job_deadline: SimTime,
    ) {
        self.jobs_done += 1;

        // Check if deadline was missed
        if completion_time > job_deadline {
            self.jobs_with_missed_deadlines += 1;
        }

        // Get job stats and task id
        let job_stat = self.event_id_to_job_stat_descr.get(&job_event_id).unwrap();
        let response_time = completion_time - job_stat.arrival_time;
        let tardiness = (completion_time - job_deadline).max(0);
        let task_event_id = job_stat.task_event_id;

        // Update task statistics
        let task_stat = self.event_id_to_task_stat_descr.get_mut(&task_event_id).unwrap();
        task_stat.add_job(response_time, tardiness);
    }

    /// Get the fraction of missed deadlines
    pub fn get_deadline_miss_ratio(&self) -> f64 {
        if self.jobs_done == 0 {
            0.0
        } else {
            self.jobs_with_missed_deadlines as f64 / self.jobs_done as f64
        }
    }

    /// Get statistics for a specific task
    pub fn get_task_stats(&self, task_event_id: u64) -> Option<&TaskStatDescriptor> {
        self.event_id_to_task_stat_descr.get(&task_event_id)
    }

    /// Record CPU transition from idle to busy. Called when a job is scheduled on CPU.
    pub fn cpu_goes_busy(&mut self, current_time: SimTime) {
        self.last_time_used = Some(current_time);
    }

    /// Record CPU transition from busy to idle. Called when CPU becomes idle.
    /// Computes elapsed time since last busy and adds to total CPU time used.
    pub fn cpu_goes_idle(&mut self, current_time: SimTime) {
        let last_time = self.last_time_used.unwrap();
        self.cpu_time_used += current_time - last_time;
        self.last_time_used = None;
    }

    /// Get total CPU time used during simulation
    pub fn get_cpu_time_used(&self) -> SimTime {
        self.cpu_time_used
    }

    /// Get CPU usage percentage (0-100)
    pub fn get_cpu_usage(&self, simulation_end_time: SimTime) -> f64 {
        if simulation_end_time == 0 {
            0.0
        } else {
            (self.cpu_time_used as f64 / simulation_end_time as f64) * 100.0
        }
    }

    /// Called at the end of the simulation. It sets the total simulation time (used to
    /// compute CPU usage) and if a job is in cpu it updates the cpu usage time
    pub fn finalize(&mut self, current_time: SimTime) {
        // If CPU is still running a job at simulation end, account for that time
        if self.last_time_used.is_some() {
            self.cpu_goes_idle(current_time);
        }
        self.simulation_end_time = Some(current_time);
    }

    /// Print a summary of all statistics
    pub fn print_summary(&self) {
        println!("\n========== SIMULATION STATISTICS ==========");
        println!("Total jobs completed: {}", self.jobs_done);
        println!("Jobs with missed deadlines: {}", self.jobs_with_missed_deadlines);
        println!("Deadline miss ratio: {:.4}", self.get_deadline_miss_ratio());

        if let Some(end_time) = self.simulation_end_time {
            println!("\n---------- CPU USAGE ----------");
            println!("CPU time used: {} / {} = {:.2}%",
                     self.cpu_time_used,
                     end_time,
                     self.get_cpu_usage(end_time));
        }

        println!("\n---------- PER-TASK STATISTICS ----------");

        for (task_event_id, task_stat) in &self.event_id_to_task_stat_descr {
            println!("Task (event_id={})", task_event_id);
            println!("  Number of jobs: {}", task_stat.num_jobs);
            println!("  Average response time: {:.2}", task_stat.avg_response_time);
            println!("  Average tardiness: {:.2}", task_stat.avg_tardiness);

            if !task_stat.response_times.is_empty() {
                let min_response = task_stat.response_times.iter().min().unwrap();
                let max_response = task_stat.response_times.iter().max().unwrap();
                println!("  Response time range: [{}, {}]", min_response, max_response);
            }
        }
        println!("==========================================\n");
    }

    /// Dump response times for all tasks to a CSV file
    pub fn dump_response_times_to_file(&self, filename: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;

        for (task_event_id, task_stat) in &self.event_id_to_task_stat_descr {
            if !task_stat.response_times.is_empty() {
                // Write header for this task
                writeln!(file, "# Task (event_id={})", task_event_id)?;

                // Write response times as CSV
                for (idx, response_time) in task_stat.response_times.iter().enumerate() {
                    writeln!(file, "{},{}", idx, response_time)?;
                }

                // Empty line between tasks
                writeln!(file)?;
            }
        }

        println!("Response times dumped to '{}'", filename);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_creation() {
        let stats = Stats::new();
        assert_eq!(stats.jobs_done, 0);
        assert_eq!(stats.jobs_with_missed_deadlines, 0);
    }

    #[test]
    fn test_register_task() {
        let mut stats = Stats::new();
        stats.register_task(1, true);
        assert!(stats.event_id_to_task_stat_descr.contains_key(&1));
    }

    #[test]
    fn test_record_job_completion_no_miss() {
        let mut stats = Stats::new();
        stats.register_task(1, true);
        stats.register_job(100, 10, 1);

        // Job arrives at 10, completes at 150, deadline is 200 (no miss)
        stats.record_job_completion(100, 150, 200);

        assert_eq!(stats.jobs_done, 1);
        assert_eq!(stats.jobs_with_missed_deadlines, 0);

        let task_stat = stats.get_task_stats(1).unwrap();
        assert_eq!(task_stat.num_jobs, 1);
        assert_eq!(task_stat.avg_response_time, 140); // 150 - 10
        assert_eq!(task_stat.avg_tardiness, 0); // max(0, 150 - 200) = 0
    }

    #[test]
    fn test_record_job_completion_with_miss() {
        let mut stats = Stats::new();
        stats.register_task(1, true);
        stats.register_job(100, 10, 1);

        // Job arrives at 10, completes at 250, deadline is 200 (miss)
        stats.record_job_completion(100, 250, 200);

        assert_eq!(stats.jobs_done, 1);
        assert_eq!(stats.jobs_with_missed_deadlines, 1);

        let task_stat = stats.get_task_stats(1).unwrap();
        assert_eq!(task_stat.num_jobs, 1);
        assert_eq!(task_stat.avg_response_time, 240); // 250 - 10
        assert_eq!(task_stat.avg_tardiness, 50); // max(0, 250 - 200) = 50
    }

    #[test]
    fn test_deadline_miss_ratio() {
        let mut stats = Stats::new();
        stats.register_task(1, true);

        stats.register_job(100, 10, 1);
        stats.record_job_completion(100, 150, 200); // No miss

        stats.register_job(101, 20, 1);
        stats.record_job_completion(101, 250, 200); // Miss

        stats.register_job(102, 30, 1);
        stats.record_job_completion(102, 250, 200); // Miss

        assert_eq!(stats.jobs_done, 3);
        assert_eq!(stats.jobs_with_missed_deadlines, 2);
        assert!((stats.get_deadline_miss_ratio() - (2.0 / 3.0)).abs() < 0.0001);
    }
}
