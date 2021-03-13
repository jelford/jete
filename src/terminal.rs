use crate::display::Display;
use crate::state::{Mode, StateSnapshot};
use crate::userinput::{Event, UserInputSource};
use crate::highlight::HighlightState;
use std::{
    io::{stdin, stdout, Stdin, Stdout, Write},
    usize,
};
use termion::{clear, color, cursor, input::{Events, MouseTerminal, TermRead}, raw::{IntoRawMode, RawTerminal}};

pub fn terminal_display() -> (TerminalDisplay, TerminalInput) {
    assert!(
        termion::is_tty(&0) && termion::is_tty(&1),
        "Not in a terminal"
    );
    let mut stdout = MouseTerminal::from(
        stdout()
            .into_raw_mode()
            .expect("Unable to set terminal to raw mode... is this a tty?"),
    );
    log::debug!("Terminal entered raw mode");
    let stdin = stdin();

    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))
        .expect("Unable to initialize display: couldn't write to stdout");
    stdout.flush().unwrap();

    (
        TerminalDisplay {
            top_line: 0,
            stdout,
        },
        TerminalInput {
            events: stdin.events(),
        },
    )
}

pub struct TerminalDisplay {
    top_line: usize,
    stdout: MouseTerminal<RawTerminal<Stdout>>,
}
pub struct TerminalInput {
    events: Events<Stdin>,
}

impl Display for TerminalDisplay {
    fn update(&mut self, state: &StateSnapshot) {
        log::debug!("Render start");
        let (w, h) = termion::terminal_size().expect("unable to check terminal dimensions");

        let lines_at_bottom = 2u16;
        let text_view_height = h - lines_at_bottom;
        let cursor_pos = state.cursor_pos();

        self.top_line = cursor_pos
            .line_number
            .saturating_sub((text_view_height as usize).saturating_sub(1));

        let hlstate = state.annotations().get::<HighlightState>();
        let text = state.text();

        let mut text_lines = text.iter_line_range(self.top_line, self.top_line.saturating_add(text_view_height as usize));
        let mut output_line = 1;

       
        while output_line < text_view_height {
            match text_lines.next() {
                Some(line) => {
                    let txt = line.content_str();
                    let escaped = hlstate.and_then(|hl| hl.highlighted_line(&line)).unwrap_or(&txt);
                    write!(
                        self.stdout,
                        "{}{}{}{:2}|{}",
                        color::Fg(color::Reset),
                        cursor::Goto(1, output_line),
                        clear::CurrentLine,
                        line.line_number(),
                        &escaped
                    )

                },
                None => { 
                    write!(
                        self.stdout,
                        "{}{}{}{:2}|~",
                        color::Fg(color::Reset),
                        cursor::Goto(1, output_line),
                        clear::CurrentLine,
                        self.top_line.saturating_add(output_line as usize - 1)
                    )
                }
            }.expect("Unable to write to main text area");
            output_line += 1;
        }


        write!(
            self.stdout,
            "{}{}",
            cursor::Goto(1, h - 1),
            clear::CurrentLine
        )
        .unwrap();

        if state.mode() == &Mode::Command {
            let command_text = state.command_line();
            let command_text_disp = &command_text[command_text.len().saturating_sub(w as usize)..];
            write!(
                self.stdout,
                "{}{}:{}",
                cursor::Goto(1, h),
                clear::CurrentLine,
                command_text_disp
            )
            .unwrap();
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
                cursor_pos.colmun
            )
            .unwrap();

            let display_cursor_ln =
                (1 + (cursor_pos.line_number - self.top_line) as u16).clamp(1, text_view_height);
            let display_cursor_col = (1 + cursor_pos.colmun as u16 + 3).clamp(1, w);

            write!(
                self.stdout,
                "{}",
                cursor::Goto(display_cursor_col, display_cursor_ln)
            )
            .unwrap();
        }

        self.stdout.flush().unwrap();
        log::debug!("Render finish");
    }
}

impl UserInputSource for TerminalInput {
    fn events(&mut self) -> &mut dyn Iterator<Item = Event> {
        self
    }
}

impl Iterator for TerminalInput {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        Some(
            self.events
                .next()
                .expect("Broken input pipe from stdin")
                .expect("Broken input pipe from stdin"),
        )
    }
}

impl Drop for TerminalDisplay {
    fn drop(&mut self) {
        let _ = write!(
            self.stdout,
            "{}{}",
            cursor::Goto(1, 1),
            clear::AfterCursor
        );
        let _ = self.stdout.flush();
    }
}
