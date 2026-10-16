//! # Statistics Module for Multi-Processor Multi-Queue Scheduler with Push/Pull Load Balancing

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use crate::base::prelude::SimTime;

pub struct JobStatDescriptor {
    pub arrival_time: SimTime,
    pub task_event_id: u64,
}

pub struct TaskStatDescriptor {
    pub response_times_freq: Option<HashMap<SimTime, u32>>,
    pub num_jobs: u32,
    pub avg_response_time: SimTime,
    pub avg_tardiness: SimTime,
    /// Jobs of this task still running or queued when the simulation ended whose
    /// deadline had already passed by then. Kept separate from `num_jobs` since they
    /// never completed, so they have no response time or tardiness to average in.
    pub num_of_unmet_and_uncompleted_jobs: u32,
}

impl TaskStatDescriptor {
    fn new(store_response_times: bool) -> Self {
        TaskStatDescriptor {
            response_times_freq: if store_response_times { Some(HashMap::new()) } else { None },
            num_jobs: 0,
            avg_response_time: 0,
            avg_tardiness: 0,
            num_of_unmet_and_uncompleted_jobs: 0,
        }
    }

    fn add_job(&mut self, response_time: SimTime, tardiness: SimTime) {
        if let Some(freq) = &mut self.response_times_freq {
            *freq.entry(response_time).or_insert(0) += 1;
        }
        self.num_jobs += 1;
        self.avg_response_time = (self.avg_response_time * (self.num_jobs - 1) as SimTime + response_time) / self.num_jobs as SimTime;
        self.avg_tardiness = (self.avg_tardiness * (self.num_jobs - 1) as SimTime + tardiness) / self.num_jobs as SimTime;
    }
}

pub struct Stats {
    pub jobs_done: u32,
    pub jobs_with_missed_deadlines: u32,
    /// Jobs still running or queued when the simulation ended whose deadline had
    /// already passed by then (see `record_unfinished_job`).
    pub num_of_unmet_and_uncompleted_jobs: u32,
    pub migrations: u32,
    pub event_id_to_job_stat_descr: HashMap<u64, JobStatDescriptor>,
    pub event_id_to_task_stat_descr: HashMap<u64, TaskStatDescriptor>,
    cpu_time_used: Vec<SimTime>,
    last_time_used: Vec<Option<SimTime>>,
    simulation_end_time: Option<SimTime>,
    num_cpus: usize,
}

impl Stats {
    pub fn new(num_cpus: usize) -> Self {
        Stats {
            jobs_done: 0,
            jobs_with_missed_deadlines: 0,
            num_of_unmet_and_uncompleted_jobs: 0,
            migrations: 0,
            event_id_to_job_stat_descr: HashMap::new(),
            event_id_to_task_stat_descr: HashMap::new(),
            cpu_time_used: vec![0; num_cpus],
            last_time_used: vec![None; num_cpus],
            simulation_end_time: None,
            num_cpus,
        }
    }

    pub fn register_task(&mut self, task_event_id: u64, store_response_times: bool) {
        self.event_id_to_task_stat_descr.insert(
            task_event_id,
            TaskStatDescriptor::new(store_response_times)
        );
    }

    pub fn register_job(&mut self, job_event_id: u64, arrival_time: SimTime, task_event_id: u64) {
        self.event_id_to_job_stat_descr.insert(
            job_event_id,
            JobStatDescriptor {
                arrival_time,
                task_event_id,
            },
        );
    }

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

    pub fn record_migration(&mut self) {
        self.migrations += 1;
    }

    /// Called at the end of the simulation for each job that was still running or
    /// queued (i.e. never reached `record_job_completion`). If its deadline had
    /// already passed by `simulation_horizon` it is counted, globally and per-task,
    /// as an unmet-and-uncompleted job. Jobs whose deadline hasn't arrived yet are
    /// left uncounted, since we can't say whether they would have made it.
    pub fn record_unfinished_job(
        &mut self,
        job_event_id: u64,
        job_deadline: SimTime,
        simulation_horizon: SimTime
    ) {
        if job_deadline >= simulation_horizon {
            return
        }

        self.num_of_unmet_and_uncompleted_jobs += 1;

        let task_event_id = self.event_id_to_job_stat_descr.get(&job_event_id).unwrap().task_event_id;
        let task_stat = self.event_id_to_task_stat_descr.get_mut(&task_event_id).unwrap();
        task_stat.num_of_unmet_and_uncompleted_jobs += 1;
    }

    /// Get the fraction of missed deadlines, counting both jobs that completed late
    /// and jobs that were still unfinished with an already-passed deadline at the
    /// end of the simulation.
    pub fn get_deadline_miss_ratio(&self) -> f64 {
        let total = self.jobs_done + self.num_of_unmet_and_uncompleted_jobs;
        if total == 0 {
            0.0
        } else {
            (self.jobs_with_missed_deadlines + self.num_of_unmet_and_uncompleted_jobs) as f64 / total as f64
        }
    }

    pub fn get_task_stats(&self, task_event_id: u64) -> Option<&TaskStatDescriptor> {
        self.event_id_to_task_stat_descr.get(&task_event_id)
    }

    pub fn cpu_goes_busy(&mut self, cpu_idx: usize, current_time: SimTime) {
        self.last_time_used[cpu_idx] = Some(current_time);
    }

    pub fn cpu_goes_idle(&mut self, cpu_idx: usize, current_time: SimTime) {
        let last_time = self.last_time_used[cpu_idx].unwrap();
        self.cpu_time_used[cpu_idx] += current_time - last_time;
        self.last_time_used[cpu_idx] = None;
    }

    pub fn get_total_cpu_time_used(&self) -> SimTime {
        self.cpu_time_used.iter().sum()
    }

    pub fn get_cpu_time_used(&self, cpu_idx: usize) -> SimTime {
        self.cpu_time_used[cpu_idx]
    }

    pub fn get_total_cpu_usage(&self, simulation_end_time: SimTime) -> f64 {
        if simulation_end_time == 0 {
            0.0
        } else {
            let total_available = (simulation_end_time as f64) * (self.num_cpus as f64);
            (self.get_total_cpu_time_used() as f64 / total_available) * 100.0
        }
    }

    pub fn get_cpu_usage(&self, cpu_idx: usize, simulation_end_time: SimTime) -> f64 {
        if simulation_end_time == 0 {
            0.0
        } else {
            (self.cpu_time_used[cpu_idx] as f64 / simulation_end_time as f64) * 100.0
        }
    }

    pub fn finalize(&mut self, current_time: SimTime) {
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
        println!("Unfinished jobs with missed deadline at simulation end: {}", self.num_of_unmet_and_uncompleted_jobs);
        println!("Deadline miss ratio: {:.4}", self.get_deadline_miss_ratio());
        println!("Total migrations: {}", self.migrations);

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
            println!("  Unfinished jobs with missed deadline: {}", task_stat.num_of_unmet_and_uncompleted_jobs);
            let missing_response_value = -1 as SimTime;
            if let Some(freq) = &task_stat.response_times_freq {
                let min_response = freq.keys().min().unwrap_or(&missing_response_value);
                let max_response = freq.keys().max().unwrap_or(&missing_response_value);
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
        assert_eq!(stats.migrations, 0);
    }

    #[test]
    fn test_migration_counter() {
        let mut stats = Stats::new(4);
        stats.record_migration();
        stats.record_migration();
        assert_eq!(stats.migrations, 2);
    }

    #[test]
    fn test_record_unfinished_job_with_passed_deadline() {
        let mut stats = Stats::new(2);
        stats.register_task(1, true);
        stats.register_job(100, 10, 1);

        // Deadline (200) already passed by simulation_horizon (300)
        stats.record_unfinished_job(100, 200, 300);

        assert_eq!(stats.jobs_done, 0);
        assert_eq!(stats.jobs_with_missed_deadlines, 0);
        assert_eq!(stats.num_of_unmet_and_uncompleted_jobs, 1);
        let task_stat = stats.get_task_stats(1).unwrap();
        assert_eq!(task_stat.num_jobs, 0);
        assert_eq!(task_stat.num_of_unmet_and_uncompleted_jobs, 1);
    }

    #[test]
    fn test_record_unfinished_job_with_deadline_not_yet_due() {
        let mut stats = Stats::new(2);
        stats.register_task(1, true);
        stats.register_job(100, 10, 1);

        // Deadline (400) hasn't arrived yet by simulation_horizon (300)
        stats.record_unfinished_job(100, 400, 300);

        assert_eq!(stats.num_of_unmet_and_uncompleted_jobs, 0);
        let task_stat = stats.get_task_stats(1).unwrap();
        assert_eq!(task_stat.num_of_unmet_and_uncompleted_jobs, 0);
    }

    #[test]
    fn test_deadline_miss_ratio_includes_unfinished_jobs() {
        let mut stats = Stats::new(2);
        stats.register_task(1, true);

        stats.register_job(100, 10, 1);
        stats.record_job_completion(100, 150, 200); // No miss

        stats.register_job(101, 20, 1);
        stats.record_unfinished_job(101, 200, 300); // Unfinished, deadline passed

        assert_eq!(stats.jobs_done, 1);
        assert_eq!(stats.num_of_unmet_and_uncompleted_jobs, 1);
        assert!((stats.get_deadline_miss_ratio() - 0.5).abs() < 0.0001);
    }
}
