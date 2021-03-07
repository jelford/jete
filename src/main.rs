use jete::editor;
use jete::state;
use jete::terminal::terminal_display;
use std::{borrow::Cow, env};

fn main() {
    let mut args = env::args_os();
    args.next().unwrap(); // safe: just the process name

    let state = match args.next() {
        None => state::empty(),
        Some(fname) => state::from_file(&fname).expect("Unable to read file"),
    };

    let (display, inputs) = terminal_display();

    editor::run(state, display, inputs)
}
