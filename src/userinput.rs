pub use termion::event::{Event, Key, MouseEvent};
pub trait UserInputSource {
    fn events(&mut self) -> &mut dyn Iterator<Item = Event>;
}
