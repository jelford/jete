

use termion::{clear, cursor, event::{Key, Event, MouseEvent}};
use termion::input::{TermRead, MouseTerminal};

use std::io::{Write, stdout, stdin, Read};
use std::{thread, writeln};
use std::time::Duration;
use jete::{display::Display, state, terminal::terminal_display, userinput::UserInputSource};

const short_time: Duration = Duration::from_millis(500);

fn main() {
    assert!(termion::is_tty(&0) && termion::is_tty(&1));

    let mut s = state::empty();
    let (mut display, mut inputs) = terminal_display();
    

    for c in inputs.events() {
        match c.unwrap() {
            Event::Key(Key::Char('q')) => break,
            Event::Key(Key::Char(c)) => s.insert(c),
            _ => {},
        }

        display.update(&s);
    }
}
