use crate::{ReadyState, Registry, TimerId, Token};

pub trait EventHandler {
    fn on_event(&mut self, registry: &Registry, token: Token, interest: ReadyState);
    fn on_timer(&mut self, registry: &Registry, timer_id: TimerId);
    fn on_wake(&mut self, registry: &Registry);
}
