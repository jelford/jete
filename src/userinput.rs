pub use termion::event::{Event, Key, MouseEvent};
pub trait UserInputSource: Send + 'static {
    fn events(&mut self) -> &mut dyn Iterator<Item = Event>;
}
