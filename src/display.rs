use crate::state::{StateSnapshot};

pub trait Display: Send + 'static {
    fn update(&mut self, state: &StateSnapshot);
}
