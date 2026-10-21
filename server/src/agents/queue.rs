use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};

use crate::agents::capabilities::CapabilitySet;

/// A single agent job enqueued for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJob {
    pub run_id: String,
    pub agent_id: String,
    pub input: String,
    pub capabilities: Vec<String>,
}

/// A chunk of streaming output from an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputChunk {
    pub run_id: String,
    pub kind: OutputKind,
    pub data: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Stdout,
    Stderr,
    Complete,
    Error,
}

/// Handle for sending streaming output from an agent run.
pub type RunBroadcast = broadcast::Sender<OutputChunk>;

/// Shared agent task queue — producers enqueue jobs, workers consume them.
#[derive(Clone)]
pub struct AgentQueue {
    tx: mpsc::Sender<AgentJob>,
    /// Broadcast channel for streaming output chunks to UI subscribers.
    output_tx: Arc<broadcast::Sender<OutputChunk>>,
}

impl AgentQueue {
    /// Create a new queue with `workers` consumer slots.
    pub fn new(buffer_size: usize) -> (Self, mpsc::Receiver<AgentJob>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let (output_tx, _) = broadcast::channel(1024);
        (
            Self {
                tx,
                output_tx: Arc::new(output_tx),
            },
            rx,
        )
    }

    /// Enqueue a job for execution.
    pub async fn enqueue(&self, job: AgentJob) -> Result<(), String> {
        self.tx
            .send(job)
            .await
            .map_err(|e| format!("queue send failed: {e}"))
    }

    /// Subscribe to streaming output for all runs.
    pub fn subscribe_output(&self) -> broadcast::Receiver<OutputChunk> {
        self.output_tx.subscribe()
    }

    /// Publish an output chunk (called by workers).
    pub fn publish_output(&self, chunk: OutputChunk) {
        // Ignore errors if no subscribers.
        let _ = self.output_tx.send(chunk);
    }

    /// Get the capability set for a job from its capability strings.
    pub fn job_capabilities(job: &AgentJob) -> CapabilitySet {
        CapabilitySet::from_strings(&job.capabilities).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_and_receive() {
        let (queue, mut rx) = AgentQueue::new(16);
        let job = AgentJob {
            run_id: "run-1".into(),
            agent_id: "agent-1".into(),
            input: "hello".into(),
            capabilities: vec!["read_docs".into()],
        };
        queue.enqueue(job.clone()).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.run_id, "run-1");
        assert_eq!(received.agent_id, "agent-1");
    }

    #[tokio::test]
    async fn test_output_broadcast() {
        let (queue, _rx) = AgentQueue::new(16);
        let mut sub = queue.subscribe_output();

        let chunk = OutputChunk {
            run_id: "run-1".into(),
            kind: OutputKind::Stdout,
            data: "hello world".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
        };
        queue.publish_output(chunk.clone());

        let received = sub.recv().await.unwrap();
        assert_eq!(received.data, "hello world");
    }
}
