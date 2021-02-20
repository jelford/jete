use crate::display::Display;
use crate::userinput::{UserInputSource};
use crate::state::{self, EditorAction, State};



pub fn run<Disp: Display, Inputs: UserInputSource>(mut s: State, mut d: Disp, mut i: Inputs) {
    
    d.update(&s);

    for e in i.events() {
        match s.dispatch(e) {
            EditorAction::Quit => break,
            _ => {}
        }

        d.update(&s);
    }
}