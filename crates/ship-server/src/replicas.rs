//! Published replicas: the latest state and a bounded every-revision feed.

use std::{ops::ControlFlow, sync::Arc};

use ship_core::{
    protocol::Replica,
    relay::{SendResult, Sink},
};
use tokio::sync::{broadcast, watch};

/// Revisions an every-revision stream can fall behind before it is lost.
pub(crate) const RING: usize = 256;

#[derive(Clone)]
pub struct Replicas {
    latest: watch::Receiver<Arc<Replica>>,
    // Only the bus sink keeps publication alive, so shutdown closes both feeds.
    ring: broadcast::WeakSender<Arc<Replica>>,
}

impl Replicas {
    pub(crate) fn new(first: Arc<Replica>) -> (Self, ReplicaSink) {
        let (latest_tx, latest) = watch::channel(first);
        let (ring, _) = broadcast::channel(RING);
        (
            Self {
                latest,
                ring: ring.downgrade(),
            },
            ReplicaSink {
                latest: latest_tx,
                ring,
            },
        )
    }

    pub fn latest(&self) -> watch::Receiver<Arc<Replica>> {
        self.latest.clone()
    }

    /// Subscribe before the attach commit. None once publication has stopped.
    pub(crate) fn every(&self) -> Option<broadcast::Receiver<Arc<Replica>>> {
        Some(self.ring.upgrade()?.subscribe())
    }
}

pub(crate) struct ReplicaSink {
    latest: watch::Sender<Arc<Replica>>,
    ring: broadcast::Sender<Arc<Replica>>,
}

impl Sink<Arc<Replica>> for ReplicaSink {
    fn try_send(&mut self, value: Arc<Replica>) -> ControlFlow<SendResult, SendResult> {
        // No ring readers is normal, not a closed replica sink.
        self.ring.send(Arc::clone(&value)).ok();
        self.latest.try_send(value)
    }
}
