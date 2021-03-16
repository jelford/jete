pub use crate::pubsub::{typed_topic, TopicId};
pub use termion::event::{Event, Key, MouseEvent};
pub trait UserInputSource: Send + 'static {
    fn events(&mut self) -> &mut dyn Iterator<Item = Event>;
}

pub fn topic() -> TopicId<Event> {
    typed_topic("input")
}
