

use termion::{clear, cursor, input::{Events, MouseTerminal, TermRead}, raw::{IntoRawMode, RawTerminal}};
use std::{io::{Stdin, Stdout, stdin, stdout, Write}, usize};
use crate::display::Display;
use crate::state::{State, Mode};
use crate::userinput::{UserInputSource, Event};

pub fn terminal_display() -> (TerminalDisplay, TerminalInput) {
    assert!(termion::is_tty(&0) && termion::is_tty(&1), "Not in a terminal");
    let mut stdout = MouseTerminal::from(stdout().into_raw_mode().expect("Unable to set terminal to raw mode... is this a tty?"));
    let stdin = stdin();

    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1)).expect("Unable to initialize display: couldn't write to stdout");
    stdout.flush().unwrap();

    (TerminalDisplay {top_line: 0, stdout}, TerminalInput{ events: stdin.events()})
}


pub struct TerminalDisplay {
    top_line: usize,
    stdout: MouseTerminal<RawTerminal<Stdout>>,
}
pub struct TerminalInput {
    events: Events<Stdin>,
}

impl Display for TerminalDisplay {
    fn update(&mut self, state: &State) {
        let (w, h) = termion::terminal_size().expect("unable to check terminal dimensions");

        let lines_at_bottom = 2 as u16;
        let text_view_height = h - lines_at_bottom;
        let cursor_pos = state.cursor_pos();

        self.top_line = cursor_pos.line_number.saturating_sub((text_view_height as usize).saturating_sub(1));

        for i in 1..=text_view_height {
            let text_line = self.top_line + (i as usize - 1);
            let output_line = i as u16;
            let line = state.line_text(text_line);
            match line {
                None => write!(self.stdout, "{}{}{:2}|~", cursor::Goto(1, output_line), clear::CurrentLine, text_line),
                Some(txt) => write!(self.stdout, "{}{}{:2}|{}", cursor::Goto(1, output_line), clear::CurrentLine, text_line, &txt[..txt.len().min(w as usize-1)]),
            }.expect("Unable to write to screen");
            write!(self.stdout, "\n\r").unwrap();
        }

        write!(
            self.stdout,
            "{}{}",
            cursor::Goto(1, h-1),
            clear::CurrentLine
        ).unwrap();

        if state.mode() == &Mode::Command {
            let command_text = state.command_line();
            let command_text_disp = &command_text[command_text.len().saturating_sub(w as usize)..];
            write!(
                self.stdout,
                "{}{}:{}",
                cursor::Goto(1, h),
                clear::CurrentLine,
                command_text_disp
            ).unwrap();


        } else {
            let status_text = state.status_text();
            let status_text_disp = &status_text[..status_text.len().min(w as usize - 1)];
            write!(
                self.stdout,
                "{}{}{}\t{:?}\t(l:{},c:{})", 
                cursor::Goto(1, h), 
                clear::CurrentLine, 
                status_text_disp, 
                state.mode(),
                cursor_pos.line_number, 
                cursor_pos.colmun).unwrap();
            
            let display_cursor_ln = (1 + (cursor_pos.line_number - self.top_line) as u16).clamp(1, text_view_height);
            let display_cursor_col = (1 + cursor_pos.colmun as u16 + 3).clamp(1, w);
    
            write!(self.stdout, "{}", 
                cursor::Goto(
                    display_cursor_col, 
                    display_cursor_ln
                )).unwrap();
        }

        self.stdout.flush().unwrap();
    }
}

impl UserInputSource for TerminalInput {
    fn events(&mut self) -> &mut dyn Iterator<Item=Event> {
        self
    }
}

impl Iterator for TerminalInput {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        Some(self.events.next().expect("Broken input pipe from stdin").expect("Broken input pipe from stdin"))
    }
}

impl Drop for TerminalDisplay {
    fn drop(&mut self) {
        // let _ = write!(
        //     self.stdout,
        //     "{}{}",
        //     cursor::Goto(1, 1),
        //     clear::AfterCursor
        // );
        // let _ = self.stdout.flush();
    }
}