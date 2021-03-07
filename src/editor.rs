use crate::display::Display;
use crate::state::{input_map, EditorAction, State};

use crate::userinput::UserInputSource;

pub fn run<Disp: Display, Inputs: UserInputSource>(mut s: State, mut d: Disp, mut i: Inputs) {
    d.update(&s);

    for e in i.events() {
        if let Some(command) = input_map(s.mode(), e) {
            if let EditorAction::Quit = s.dispatch(command) {
                break;
            }
        }

        d.update(&s);
    }
}
