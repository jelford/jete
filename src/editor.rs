use crate::display::Display;
use crate::state::{EditorAction, State};
use crate::userinput::UserInputSource;

pub fn run<Disp: Display, Inputs: UserInputSource>(mut s: State, mut d: Disp, mut i: Inputs) {
    d.update(&s);

    for e in i.events() {
        if let EditorAction::Quit = s.dispatch(e) {
            break;
        }

        d.update(&s);
    }
}
