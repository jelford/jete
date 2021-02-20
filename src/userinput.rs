
pub trait UserInputSource {
    type InputEventType;
    fn events(&mut self) -> &mut Self::InputEventType;
}
