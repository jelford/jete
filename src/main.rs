

use termion::{clear, cursor, event::{Key, Event, MouseEvent}};
use termion::input::{TermRead, MouseTerminal};

use std::io::{Write, stdout, stdin, Read};
use std::{thread, writeln};
use std::time::Duration;
use jete::{display::Display, state, terminal::terminal_display, userinput::UserInputSource, editor};

const short_time: Duration = Duration::from_millis(500);

fn main() {
    assert!(termion::is_tty(&0) && termion::is_tty(&1));

    let (mut display, mut inputs) = terminal_display();
    
    editor::run(display, inputs)
}
