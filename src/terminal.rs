


use clear::CurrentLine;
use termion::{clear, cursor, input::{Events, MouseTerminal, TermRead}, raw::{IntoRawMode, RawTerminal}};
use std::{io::{Stdin, Stdout, stdin, stdout, Write}, usize};
use crate::display::Display;
use crate::state::State;
use crate::userinput::UserInputSource;

pub fn terminal_display() -> (TerminalDisplay, TerminalInput) {
    let stdout = MouseTerminal::from(stdout().into_raw_mode().expect("Unable to set terminal to raw mode... is this a tty?"));
    let stdin = stdin();

    (TerminalDisplay {stdout}, TerminalInput{ events: stdin.events()})
}


pub struct TerminalDisplay {
    stdout: MouseTerminal<RawTerminal<Stdout>>,
}
pub struct TerminalInput {
    events: Events<Stdin>,
}

impl Display for TerminalDisplay {
    fn update(&mut self, state: &State) {
        write!(self.stdout, "{}{}", cursor::Goto(1, 1), clear::All).expect("Unable to write to screen");
        let (w, h) = termion::terminal_size().expect("unable to check terminal dimensions");

        for i in 1..=h-2 {
            let output_line = i;
            let line = state.line_text((i-1) as usize);
            match line {
                None => write!(self.stdout, "{}{}~{}", cursor::Goto(1, output_line), clear::CurrentLine, if i < h { "\n\r" } else { "" }),
                Some(txt) => write!(self.stdout, "{}{}{}", cursor::Goto(1, output_line), clear::CurrentLine, &txt[..txt.len().min(w as usize-1)]),
            }.expect("Unable to write to screen");
        }

        let status_text = state.status_text();
        let status_text_disp = &status_text[..status_text.len().min(w as usize - 1)];
        write!(self.stdout, "{}{}{}", cursor::Goto(1, h), clear::CurrentLine, status_text_disp).unwrap();

        self.stdout.flush().unwrap();
    }
}

pub use termion::event::{Event, Key, MouseEvent, MouseButton};


impl UserInputSource for TerminalInput {
    type InputEventType = Events<Stdin>;

    fn events(&mut self) -> &mut Self::InputEventType {
        &mut self.events
    }
}