//! # Minimal Statistics Module for Single-Processor Scheduler
//!
//! Lightweight stats collection for fair performance comparison with C++.
//! Only tracks response times, no averages or other metrics computed during simulation.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use crate::base::prelude::SimTime;

/// Minimal job stat descriptor - only arrival time needed for response time calculation
pub struct JobStatDescriptor {
    pub arrival_time: SimTime,
    pub task_event_id: u64,
}

/// Minimal task stat descriptor - only stores response time frequency
pub struct TaskStatDescriptor {
    pub response_times_freq: HashMap<SimTime, u32>,
}

impl TaskStatDescriptor {
    fn new() -> Self {
        TaskStatDescriptor {
            response_times_freq: HashMap::new(),
        }
    }

    fn record_response_time(&mut self, response_time: SimTime) {
        *self.response_times_freq.entry(response_time).or_insert(0) += 1;
    }
}

/// Minimal statistics struct - only tracks response times
pub struct Stats {
    pub jobs_done: u32,
    pub event_id_to_job_stat_descr: HashMap<u64, JobStatDescriptor>,
    pub event_id_to_task_stat_descr: HashMap<u64, TaskStatDescriptor>,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            jobs_done: 0,
            event_id_to_job_stat_descr: HashMap::new(),
            event_id_to_task_stat_descr: HashMap::new(),
        }
    }

    pub fn register_task(&mut self, task_event_id: u64, _store_response_times: bool) {
        self.event_id_to_task_stat_descr.insert(
            task_event_id,
            TaskStatDescriptor::new()
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
        _job_deadline: SimTime,
    ) {
        self.jobs_done += 1;
        let job_stat = self.event_id_to_job_stat_descr.get(&job_event_id).unwrap();
        let response_time = completion_time - job_stat.arrival_time;
        let task_event_id = job_stat.task_event_id;
        let task_stat = self.event_id_to_task_stat_descr.get_mut(&task_event_id).unwrap();
        task_stat.record_response_time(response_time);
    }

    pub fn cpu_goes_busy(&mut self, _current_time: SimTime) {}
    pub fn cpu_goes_idle(&mut self, _current_time: SimTime) {}

    pub fn finalize(&mut self, _current_time: SimTime) {}

    pub fn print_summary(&self) {
        println!("\n========== SIMULATION STATISTICS ==========");
        println!("Total jobs completed: {}", self.jobs_done);
        println!("==========================================\n");
    }

    pub fn dump_response_times_to_file(&self, filename: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        for (task_event_id, task_stat) in &self.event_id_to_task_stat_descr {
            if task_stat.response_times_freq.is_empty() {
                continue;
            }
            writeln!(file, "# Task (event_id={})", task_event_id)?;
            let mut sorted_times: Vec<_> = task_stat.response_times_freq.iter().collect();
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
        let stats = Stats::new();
        assert_eq!(stats.jobs_done, 0);
    }

    #[test]
    fn test_record_job_completion() {
        let mut stats = Stats::new();
        stats.register_task(1, true);
        stats.register_job(100, 10, 1);
        stats.record_job_completion(100, 150, 200);
        assert_eq!(stats.jobs_done, 1);
        let task_stat = stats.event_id_to_task_stat_descr.get(&1).unwrap();
        assert_eq!(task_stat.response_times_freq.get(&140), Some(&1));
    }
}
