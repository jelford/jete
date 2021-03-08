use crate::state::State;

pub trait Display: Send {
    fn update(&mut self, state: &State);
}
