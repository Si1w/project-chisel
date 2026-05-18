use tokio::sync::{broadcast, mpsc};

use crate::event::payload::{
    CommandAckEvent, CommandEvent, DomainEvent, InputEvent, MarkerEvent, PresentationCommand,
    SnapshotEvent,
};

/// Errors a bus operation can surface.
#[derive(thiserror::Error, Debug)]
pub enum BusError {
    #[error("bus closed")]
    Closed,
    #[error("subscriber lagged by {0} messages")]
    Lagged(u64),
}

/// All outbound + inbound senders bundled together. Cheap to clone —
/// every field is itself a wrapped sender handle.
#[derive(Clone)]
pub struct Bus {
    pub input: InboundTx<InputEvent>,
    pub command: InboundTx<CommandEvent>,
    pub domain: OutboundChannel<DomainEvent>,
    pub marker: OutboundChannel<MarkerEvent>,
    pub presentation: OutboundChannel<PresentationCommand>,
    pub command_ack: OutboundChannel<CommandAckEvent>,
    pub snapshot: OutboundChannel<SnapshotEvent>,
}

/// Single-consumer receivers for inbound channels. Handed out exactly
/// once at construction; subsequent code cannot subscribe to inbound
/// streams (architecture invariant — one consumer per inbound channel).
pub struct BusEndpoints {
    pub input_rx: InboundRx<InputEvent>,
    pub command_rx: InboundRx<CommandEvent>,
}

impl Bus {
    /// `inbound_capacity`: per-mpsc bound that back-pressures the stdin
    /// reader when full.
    ///
    /// `outbound_capacity`: per-subscriber broadcast lag tolerance.
    #[must_use]
    pub fn new(_inbound_capacity: usize, _outbound_capacity: usize) -> (Self, BusEndpoints) {
        todo!()
    }
}

/// Inbound (mpsc) sender wrapper. Cloneable so multiple producers can
/// push to the same single-consumer queue.
#[derive(Clone)]
pub struct InboundTx<T> {
    inner: mpsc::Sender<T>,
}

impl<T> InboundTx<T> {
    /// # Errors
    ///
    /// Returns `BusError::Closed` if the matching `InboundRx` has been
    /// dropped.
    pub async fn send(&self, _event: T) -> Result<(), BusError> {
        todo!()
    }
}

/// Inbound (mpsc) receiver. Held by exactly one task — the input mapper
/// for `input`, the command handler for `command`.
pub struct InboundRx<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> InboundRx<T> {
    pub async fn recv(&mut self) -> Option<T> {
        todo!()
    }
}

/// Outbound (broadcast) channel. The sender handle is clone-shared; each
/// `subscribe()` returns a fresh receiver.
pub struct OutboundChannel<T: Clone> {
    inner: broadcast::Sender<T>,
}

impl<T: Clone> Clone for OutboundChannel<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Clone + Send + 'static> OutboundChannel<T> {
    /// # Errors
    ///
    /// Returns `BusError::Closed` when no subscribers remain.
    pub fn emit(&self, _event: T) -> Result<(), BusError> {
        todo!()
    }

    #[must_use]
    pub fn subscribe(&self) -> OutboundRx<T> {
        todo!()
    }
}

pub struct OutboundRx<T> {
    inner: broadcast::Receiver<T>,
}

impl<T: Clone + Send + 'static> OutboundRx<T> {
    /// # Errors
    ///
    /// Returns `BusError::Lagged(n)` if `n` messages were dropped before
    /// this receiver caught up; the caller decides whether to reconnect.
    /// Returns `BusError::Closed` once all senders are dropped.
    pub async fn recv(&mut self) -> Result<T, BusError> {
        todo!()
    }
}
