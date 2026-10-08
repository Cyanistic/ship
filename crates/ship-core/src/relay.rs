use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    ops::ControlFlow,
};

use kameo::prelude::*;
use ship_macros::Actor;
use tokio::sync::{mpsc, watch};

#[derive(Actor, Default)]
pub struct RelayBus {
    subscriptions: HashMap<TypeId, Vec<Box<dyn Any + Send>>>,
}

/// Subscribe a typed sink to receive publications of type `T`.
pub struct Subscribe<T> {
    pub sink: Box<dyn Sink<T> + 'static>,
}

/// Clone `T` into every sink subscribed to it.
pub struct Publish<T>(pub T);

impl<T: Clone + Send + 'static> Message<Subscribe<T>> for RelayBus {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Subscribe<T>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let type_id = TypeId::of::<T>();
        self.subscriptions
            .entry(type_id)
            .or_default()
            .push(Box::new(msg.sink));
    }
}

impl<T: Clone + Send + 'static> Message<Publish<T>> for RelayBus {
    type Reply = ();

    /// A sink that answers `Break` is removed.
    async fn handle(
        &mut self,
        msg: Publish<T>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let type_id = TypeId::of::<T>();
        if let Some(sinks) = self.subscriptions.get_mut(&type_id) {
            sinks.retain_mut(|any_sink| {
                if let Some(sink) = any_sink.downcast_mut::<Box<dyn Sink<T> + 'static>>() {
                    matches!(sink.try_send(msg.0.clone()), ControlFlow::Continue(_))
                } else {
                    true
                }
            });
            if sinks.is_empty() {
                self.subscriptions.remove(&type_id);
            }
        }
    }
}

pub trait Sink<T>: Send + 'static {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult {
    Sent,
    Full,
    Closed,
}

impl<T: Send + 'static> Sink<T> for mpsc::Sender<T> {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult> {
        match mpsc::Sender::try_send(self, value) {
            Ok(()) => ControlFlow::Continue(SendResult::Sent),
            Err(mpsc::error::TrySendError::Full(_)) => ControlFlow::Continue(SendResult::Full),
            Err(mpsc::error::TrySendError::Closed(_)) => ControlFlow::Break(SendResult::Closed),
        }
    }
}

/// Never `Full`; `Closed` once the receiver is gone. For publications that
/// must not drop, such as exit statuses.
impl<T: Send + 'static> Sink<T> for mpsc::UnboundedSender<T> {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult> {
        match self.send(value) {
            Ok(()) => ControlFlow::Continue(SendResult::Sent),
            Err(_) => ControlFlow::Break(SendResult::Closed),
        }
    }
}

/// Drops the publication when the mailbox is full, so it must not carry
/// authoritative state.
impl<T: Clone + Send + 'static> Sink<T> for Recipient<T> {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult> {
        match self.tell(value).try_send() {
            Ok(()) => ControlFlow::Continue(SendResult::Sent),
            Err(SendError::ActorNotRunning(_) | SendError::ActorStopped) => {
                ControlFlow::Break(SendResult::Closed)
            }
            Err(
                SendError::ActorRestarting(_)
                | SendError::HandlerError(_)
                | SendError::Timeout(_)
                | SendError::MailboxFull(_),
            ) => ControlFlow::Continue(SendResult::Full),
        }
    }
}

/// Replaces the latest value, so it is never `Full`; `Closed` once every
/// receiver is gone. This is the sink for authoritative replicas.
impl<T: Send + Sync + 'static> Sink<T> for watch::Sender<T> {
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult> {
        match self.send(value) {
            Ok(()) => ControlFlow::Continue(SendResult::Sent),
            Err(_) => ControlFlow::Break(SendResult::Closed),
        }
    }
}

impl<T, F> Sink<T> for F
where
    F: FnMut(T) -> ControlFlow<SendResult, SendResult> + Send + 'static,
{
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult> {
        self(value)
    }
}

pub trait SinkExt<T>: Sink<T> + Sized {
    fn filter<F>(self, predicate: F) -> FilterSink<Self, F>
    where
        F: FnMut(&T) -> bool + Send + 'static,
    {
        FilterSink {
            inner: self,
            predicate,
        }
    }

    fn filter_map<U, F>(self, f: F) -> FilterMapSink<Self, U, F>
    where
        F: FnMut(U) -> Option<T> + Send + 'static,
    {
        FilterMapSink {
            inner: self,
            f,
            _phantom: PhantomData,
        }
    }
}

impl<T, S: Sink<T> + Sized> SinkExt<T> for S {}

/// Forwards values the predicate accepts; a rejected value is a deliberate no-op.
pub struct FilterSink<S, F> {
    inner: S,
    predicate: F,
}

impl<T: Send + 'static, S: Sink<T>, F: FnMut(&T) -> bool + Send + 'static> Sink<T>
    for FilterSink<S, F>
{
    fn try_send(&mut self, value: T) -> ControlFlow<SendResult, SendResult> {
        if (self.predicate)(&value) {
            self.inner.try_send(value)
        } else {
            ControlFlow::Continue(SendResult::Sent)
        }
    }
}

/// Maps each value; `None` is a deliberate no-op.
pub struct FilterMapSink<S, U, F> {
    inner: S,
    f: F,
    _phantom: PhantomData<fn(U)>,
}

impl<T: Send + 'static, U: Send + 'static, S: Sink<T>, F: FnMut(U) -> Option<T> + Send + 'static>
    Sink<U> for FilterMapSink<S, U, F>
{
    fn try_send(&mut self, value: U) -> ControlFlow<SendResult, SendResult> {
        match (self.f)(value) {
            Some(mapped) => self.inner.try_send(mapped),
            None => ControlFlow::Continue(SendResult::Sent),
        }
    }
}
