//! # Statistics Module for Multi-Processor Multi-Queue Scheduler

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
/// `response_times_freq` is an Option wrapping a HashMap storing response time -> frequency (only allocated if store_response_times is true)
/// `num_jobs` is the number of jobs, used when computing "rolling" averages
/// `avg_response_time` and `avg_tardiness` are self-explanatory
pub struct TaskStatDescriptor {
    pub response_times_freq: Option<HashMap<SimTime, u32>>,
    pub num_jobs: u32,
    pub avg_response_time: SimTime,
    pub avg_tardiness: SimTime,
}

impl TaskStatDescriptor {

    /// Initializes task statistics with Frequency HashMap if response times have to be
    /// stored otherwise with None
    fn new(store_response_times: bool) -> Self {
        TaskStatDescriptor {
            response_times_freq: if store_response_times { Some(HashMap::new()) } else { None },
            num_jobs: 0,
            avg_response_time: 0,
            avg_tardiness: 0,
        }
    }

    /// Add a job's response time and tardiness. Only stores response time frequency if HashMap was allocated.
    fn add_job(&mut self, response_time: SimTime, tardiness: SimTime) {
        if let Some(freq) = &mut self.response_times_freq {
            *freq.entry(response_time).or_insert(0) += 1;
        }
        self.num_jobs += 1;
        self.avg_response_time = (self.avg_response_time * (self.num_jobs - 1) as SimTime + response_time) / self.num_jobs as SimTime;
        self.avg_tardiness = (self.avg_tardiness * (self.num_jobs - 1) as SimTime + tardiness) / self.num_jobs as SimTime;
    }
}

/// Statistics struct for multi-processor multi-queue system with per-CPU tracking
/// `jobs_done` and `jobs_with_missed_deadlines` are used to compute ratio of not met deadlines
/// `event_id_to_X` are hash maps mapping id of the event to the task or job
/// `cpu_time_used` accumulates total time that i^th CPU spent executing jobs
/// `last_time_used` records the time when that i^th CPU transitioned from idle to busy
/// `simulation_end_time` stores the end time of simulation for that i^th CPU usage computation
pub struct Stats {
    pub jobs_done: u32,
    pub jobs_with_missed_deadlines: u32,
    pub event_id_to_job_stat_descr: HashMap<u64, JobStatDescriptor>,
    pub event_id_to_task_stat_descr: HashMap<u64, TaskStatDescriptor>,
    cpu_time_used: Vec<SimTime>,
    last_time_used: Vec<Option<SimTime>>,
    simulation_end_time: Option<SimTime>,
    num_cpus: usize,
}

impl Stats {
    /// Create a new Stats struct for multi-processor multi-queue system
    pub fn new(num_cpus: usize) -> Self {
        Stats {
            jobs_done: 0,
            jobs_with_missed_deadlines: 0,
            event_id_to_job_stat_descr: HashMap::new(),
            event_id_to_task_stat_descr: HashMap::new(),
            cpu_time_used: vec![0; num_cpus],
            last_time_used: vec![None; num_cpus],
            simulation_end_time: None,
            num_cpus,
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

        if completion_time > job_deadline {
            self.jobs_with_missed_deadlines += 1;
        }

        let job_stat = self.event_id_to_job_stat_descr.get(&job_event_id).unwrap();
        let response_time = completion_time - job_stat.arrival_time;
        let tardiness = (completion_time - job_deadline).max(0);
        let task_event_id = job_stat.task_event_id;

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

    /// Record CPU transition from idle to busy for a specific CPU
    pub fn cpu_goes_busy(&mut self, cpu_idx: usize, current_time: SimTime) {
        self.last_time_used[cpu_idx] = Some(current_time);
    }

    /// Record CPU transition from busy to idle for a specific CPU
    /// Computes elapsed time since last busy and adds to total CPU time used.
    pub fn cpu_goes_idle(&mut self, cpu_idx: usize, current_time: SimTime) {
        let last_time = self.last_time_used[cpu_idx].unwrap();
        self.cpu_time_used[cpu_idx] += current_time - last_time;
        self.last_time_used[cpu_idx] = None;
    }

    /// Get total CPU time used across all CPUs
    pub fn get_total_cpu_time_used(&self) -> SimTime {
        self.cpu_time_used.iter().sum()
    }

    /// Get CPU time used by a specific CPU
    pub fn get_cpu_time_used(&self, cpu_idx: usize) -> SimTime {
        self.cpu_time_used[cpu_idx]
    }

    /// Get total CPU usage percentage (sum of all CPUs / (simulation_time * num_cpus)) * 100
    pub fn get_total_cpu_usage(&self, simulation_end_time: SimTime) -> f64 {
        if simulation_end_time == 0 {
            0.0
        } else {
            let total_available = (simulation_end_time as f64) * (self.num_cpus as f64);
            (self.get_total_cpu_time_used() as f64 / total_available) * 100.0
        }
    }

    /// Get CPU usage percentage for a specific CPU
    pub fn get_cpu_usage(&self, cpu_idx: usize, simulation_end_time: SimTime) -> f64 {
        if simulation_end_time == 0 {
            0.0
        } else {
            (self.cpu_time_used[cpu_idx] as f64 / simulation_end_time as f64) * 100.0
        }
    }

    /// Called at the end of the simulation. It sets the total simulation time (used to
    /// compute CPU usage) and if a job is in cpu it updates the cpu usage time
    pub fn finalize(&mut self, current_time: SimTime) {
        // If any CPU is still running a job at simulation end, account for that time
        for cpu_idx in 0..self.num_cpus {
            if self.last_time_used[cpu_idx].is_some() {
                self.cpu_goes_idle(cpu_idx, current_time);
            }
        }
        self.simulation_end_time = Some(current_time);
    }

    pub fn print_summary(&self) {
        println!("\n========== SIMULATION STATISTICS ==========");
        println!("Total jobs completed: {}", self.jobs_done);
        println!("Jobs with missed deadlines: {}", self.jobs_with_missed_deadlines);
        println!("Deadline miss ratio: {:.4}", self.get_deadline_miss_ratio());

        if let Some(end_time) = self.simulation_end_time {
            println!("\n---------- CPU USAGE ----------");
            println!("Total CPU time used: {} / ({} * {}) = {:.2}%",
                     self.get_total_cpu_time_used(),
                     end_time,
                     self.num_cpus,
                     self.get_total_cpu_usage(end_time));

            println!("\nPer-CPU breakdown:");
            for cpu_idx in 0..self.num_cpus {
                println!("  CPU {}: {} / {} = {:.2}%",
                         cpu_idx,
                         self.cpu_time_used[cpu_idx],
                         end_time,
                         self.get_cpu_usage(cpu_idx, end_time));
            }
        }

        println!("\n---------- PER-TASK STATISTICS ----------");
        for (task_event_id, task_stat) in &self.event_id_to_task_stat_descr {
            println!("Task (event_id={})", task_event_id);
            println!("  Number of jobs: {}", task_stat.num_jobs);
            println!("  Average response time: {:.2}", task_stat.avg_response_time);
            println!("  Average tardiness: {:.2}", task_stat.avg_tardiness);
            if let Some(freq) = &task_stat.response_times_freq {
                let min_response = freq.keys().min().unwrap();
                let max_response = freq.keys().max().unwrap();
                println!("  Response time range: [{}, {}]", min_response, max_response);
            }
        }
        println!("==========================================\n");
    }

    pub fn dump_response_times_to_file(&self, filename: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        for (task_event_id, task_stat) in &self.event_id_to_task_stat_descr {
            let Some(freq) = &task_stat.response_times_freq else {
                continue;
            };
            writeln!(file, "# Task (event_id={})", task_event_id)?;
            let mut sorted_times: Vec<_> = freq.iter().collect();
            sorted_times.sort_by_key(|&(time, _)| time);
            for (response_time, frequency) in sorted_times {
                writeln!(file, "{},{}", response_time, frequency)?;
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
        let stats = Stats::new(4);
        assert_eq!(stats.jobs_done, 0);
        assert_eq!(stats.num_cpus, 4);
    }

    #[test]
    fn test_per_cpu_tracking() {
        let mut stats = Stats::new(2);
        stats.cpu_goes_busy(0, 0);
        stats.cpu_goes_idle(0, 10);
        assert_eq!(stats.get_cpu_time_used(0), 10);
        assert_eq!(stats.get_cpu_time_used(1), 0);
    }

    #[test]
    fn test_response_time_frequency() {
        let mut stats = Stats::new(2);
        stats.register_task(1, true);
        stats.register_job(100, 10, 1);
        stats.record_job_completion(100, 150, 200);
        let task_stat = stats.get_task_stats(1).unwrap();
        assert_eq!(task_stat.response_times_freq.as_ref().unwrap().get(&140), Some(&1));
    }
}
