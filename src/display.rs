use crate::state::State;

pub trait Display: Send + 'static {
    fn update(&mut self, state: &State);
}
