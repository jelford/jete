use crate::state::State;

pub trait Display {
    fn update(&mut self, state: &State);
}
