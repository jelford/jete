use crate::display::Display;
use crate::userinput::{UserInputSource};
use crate::state::{self, EditorAction};



pub fn run<Disp: Display, Inputs: UserInputSource>(mut d: Disp, mut i: Inputs) {
    let mut s = state::empty();
    
    d.update(&s);

    for e in i.events() {
        match s.dispatch(e) {
            EditorAction::Quit => break,
            _ => {}
        }

        d.update(&s);
    }
}