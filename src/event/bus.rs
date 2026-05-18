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
    pub fn new(inbound_capacity: usize, outbound_capacity: usize) -> (Self, BusEndpoints) {
        let (input_tx, input_rx) = mpsc::channel(inbound_capacity);
        let (command_tx, command_rx) = mpsc::channel(inbound_capacity);
        let (domain_tx, _) = broadcast::channel(outbound_capacity);
        let (marker_tx, _) = broadcast::channel(outbound_capacity);
        let (presentation_tx, _) = broadcast::channel(outbound_capacity);
        let (command_ack_tx, _) = broadcast::channel(outbound_capacity);
        let (snapshot_tx, _) = broadcast::channel(outbound_capacity);

        (
            Self {
                input: InboundTx { inner: input_tx },
                command: InboundTx { inner: command_tx },
                domain: OutboundChannel { inner: domain_tx },
                marker: OutboundChannel { inner: marker_tx },
                presentation: OutboundChannel {
                    inner: presentation_tx,
                },
                command_ack: OutboundChannel {
                    inner: command_ack_tx,
                },
                snapshot: OutboundChannel { inner: snapshot_tx },
            },
            BusEndpoints {
                input_rx: InboundRx { inner: input_rx },
                command_rx: InboundRx { inner: command_rx },
            },
        )
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
    pub async fn send(&self, event: T) -> Result<(), BusError> {
        self.inner.send(event).await.map_err(|_| BusError::Closed)
    }
}

/// Inbound (mpsc) receiver. Held by exactly one task — the input mapper
/// for `input`, the command handler for `command`.
pub struct InboundRx<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> InboundRx<T> {
    pub async fn recv(&mut self) -> Option<T> {
        self.inner.recv().await
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
    pub fn emit(&self, event: T) -> Result<(), BusError> {
        self.inner
            .send(event)
            .map(|_| ())
            .map_err(|_| BusError::Closed)
    }

    #[must_use]
    pub fn subscribe(&self) -> OutboundRx<T> {
        OutboundRx {
            inner: self.inner.subscribe(),
        }
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
        match self.inner.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Closed) => Err(BusError::Closed),
            Err(broadcast::error::RecvError::Lagged(count)) => Err(BusError::Lagged(count)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::event::payload::{AckStatus, CommandAckEvent, InputEvent};

    use super::*;

    #[tokio::test]
    async fn inbound_channel_round_trips_to_single_receiver() {
        let (bus, mut endpoints) = Bus::new(4, 4);

        bus.input
            .send(InputEvent::KeyPress {
                key: "Space".into(),
            })
            .await
            .expect("receiver is alive");

        let event = endpoints
            .input_rx
            .recv()
            .await
            .expect("event should arrive");

        match event {
            InputEvent::KeyPress { key } => assert_eq!(key, "Space"),
            InputEvent::KeyRelease { key } => panic!("unexpected key release: {key}"),
        }
    }

    #[tokio::test]
    async fn outbound_channel_broadcasts_to_subscriber() {
        let (bus, _endpoints) = Bus::new(4, 4);
        let mut rx = bus.command_ack.subscribe();

        bus.command_ack
            .emit(CommandAckEvent {
                command_id: Some("cmd-1".into()),
                status: AckStatus::Ok,
                message: None,
            })
            .expect("subscriber is alive");

        let event = rx.recv().await.expect("event should arrive");

        assert_eq!(event.command_id.as_deref(), Some("cmd-1"));
        assert_eq!(event.status, AckStatus::Ok);
    }
}
