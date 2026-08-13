use std::marker::PhantomData;

use winit::event_loop::{EventLoopClosed, EventLoopProxy};

use crate::app::state::State;

pub trait EventSender<T> {
    type Error;
    fn send_event(&self, event: T) -> Result<(), EventLoopClosed<Self::Error>>;

    fn map<U, F>(&self, map: F) -> ProxySender<Self, F, T, U>
    where
        Self: Sized + Clone,
        F: Fn(U) -> T,
    {
        ProxySender {
            sender: self.clone(),
            map,
            p: PhantomData,
        }
    }
}

impl<T: 'static> EventSender<T> for EventLoopProxy<T> {
    type Error = T;

    fn send_event(&self, event: T) -> Result<(), EventLoopClosed<Self::Error>> {
        self.send_event(event)
    }
}

#[derive(Debug, Clone)]
pub struct ProxySender<S, F, T, U>
where
    S: EventSender<T>,
    F: Fn(U) -> T,
{
    sender: S,
    map: F,
    p: PhantomData<(T, U)>,
}

impl<S, F, T, U> EventSender<U> for ProxySender<S, F, T, U>
where
    S: EventSender<T>,
    F: Fn(U) -> T,
{
    type Error = ();
    fn send_event(&self, event: U) -> Result<(), EventLoopClosed<Self::Error>> {
        self.sender
            .send_event((self.map)(event))
            .map_err(|EventLoopClosed(_)| EventLoopClosed(()))
    }
}

pub enum AppEvent {
    State(Box<State>),
    User(UserEvent),
}

pub enum UserEvent {}
