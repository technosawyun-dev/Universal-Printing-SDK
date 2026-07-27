//! In-memory print-job tracking, per docs/PROTOCOL.md's `GET /jobs/:jobId`
//! and the `job.status` WebSocket event. Jobs aren't persisted across an
//! agent restart -- they're a transient "is this still printing" view, not
//! an audit log.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Printing,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobStatus {
    #[serde(rename = "jobId")]
    pub job_id: String,
    pub status: JobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct JobRegistry {
    jobs: Mutex<HashMap<String, JobStatus>>,
}

impl JobRegistry {
    pub fn create(&self) -> String {
        let job_id = format!("job_{}", uuid::Uuid::new_v4().simple());
        self.jobs.lock().unwrap().insert(job_id.clone(), JobStatus { job_id: job_id.clone(), status: JobState::Queued, error: None });
        job_id
    }

    pub fn set(&self, job_id: &str, status: JobState, error: Option<String>) {
        if let Some(job) = self.jobs.lock().unwrap().get_mut(job_id) {
            job.status = status;
            job.error = error;
        }
    }

    pub fn get(&self, job_id: &str) -> Option<JobStatus> {
        self.jobs.lock().unwrap().get(job_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_job_in_queued_state() {
        let registry = JobRegistry::default();
        let id = registry.create();
        let job = registry.get(&id).unwrap();
        assert_eq!(job.status, JobState::Queued);
        assert!(job.error.is_none());
    }

    #[test]
    fn updates_job_status_and_error() {
        let registry = JobRegistry::default();
        let id = registry.create();
        registry.set(&id, JobState::Error, Some("printer offline".to_string()));
        let job = registry.get(&id).unwrap();
        assert_eq!(job.status, JobState::Error);
        assert_eq!(job.error.as_deref(), Some("printer offline"));
    }

    #[test]
    fn unknown_job_returns_none() {
        let registry = JobRegistry::default();
        assert!(registry.get("job_does_not_exist").is_none());
    }
}
