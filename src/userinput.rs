pub use termion::event::{Event, Key, MouseEvent};
pub use crate::pubsub::{TopicId, typed_topic};
pub trait UserInputSource: Send + 'static {
    fn events(&mut self) -> &mut dyn Iterator<Item = Event>;
}

pub fn topic() -> TopicId<Event> {
    typed_topic("input")
}