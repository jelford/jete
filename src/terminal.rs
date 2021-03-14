use crate::{pubsub, userinput::{self}};
use crate::state::{Mode, StateSnapshot, state_update_topic};
use crate::userinput::{Event};
use crate::highlight::HighlightState;
use std::{io::{stdin, stdout, Stdin, Stdout, Write}, time::{Instant, Duration}, usize};
use crossbeam::select;
use crossbeam::channel::{after, never};
use termion::{clear, color, cursor, input::{Events, TermRead}, raw::{IntoRawMode, RawTerminal}};
use std::thread;

const MILLIS_BUDGET_PER_FRAME: Duration = Duration::from_millis(16);

fn terminal_display() -> (TerminalDisplay, TerminalInput) {
    assert!(
        termion::is_tty(&0) && termion::is_tty(&1),
        "Not in a terminal"
    );
    let mut stdout = 
        stdout()
            .into_raw_mode()
            .expect("Unable to set terminal to raw mode... is this a tty?");

    log::debug!("Terminal entered raw mode");
    let stdin = stdin();
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
    stdout: RawTerminal<Stdout>,
}
pub struct TerminalInput {
    events: Events<Stdin>,
}

pub fn spawn_interface(hub: pubsub::Hub) -> thread::JoinHandle<()> {
    let (mut display, input) = terminal_display();

    let mut display_hub = hub.clone();
    let mut input_hub = hub.clone();

    let input_thread = thread::Builder::new().name("input".into()).spawn(move || {
        for e in input {
            let send_result = input_hub.send(userinput::topic(), e);
            if send_result.is_err() {
                log::debug!("Shutting down listen thread");
                // nobody is listening
                break;
            }
        }
        log::debug!("Input thread closing");
    }).expect("Failed spawning input listener thread");
    // daemonize - let it unwind when the process finishes
    drop(input_thread);


    thread::Builder::new().name("display".into()).spawn(move || {
        let update_topic = state_update_topic();
        let state_receiver = display_hub.get_receiver(update_topic);
        let shutdown_receiver = display_hub.get_receiver(crate::editor::shutdown_event_topic());

        log::debug!("Initializing display thread");

        let mut last_state = None;
        let start_time = Instant::now();
        let mut deadline: Option<Instant> = Some(start_time.checked_add(MILLIS_BUDGET_PER_FRAME).unwrap());

        log::debug!("Setting next render deadline: {:?}", deadline);

        loop {
            let now = Instant::now();
            log::debug!("It's now: {:?}. Deadline is: {:?}", now, deadline);
            if deadline.is_none() {
                log::debug!("No current deadline");
            }
            
            let time_until_deadline = 
                deadline.map(
                    |d| d.checked_duration_since(now).unwrap_or(Duration::from_millis(0)));

            select! {
                recv(shutdown_receiver) -> _ => {
                    log::debug!("Shutdown signal received");
                    break;
                }
                recv(state_receiver) -> msg => {
                    if let Err(e) = msg {
                        log::debug!("Got error down pipe: {:?}", e);
                        continue;
                    }
                    // the next frame deadline is when now_millis - start_millis % 16 == 0
                    // or whatever the current deadline is
                    last_state = Some(msg.unwrap());
                    if deadline.is_some() {
                        log::debug!("Deadline already set...");
                        continue;
                    }
                    let now = Instant::now();
                    let millis_in = now.checked_duration_since(start_time).unwrap_or(Duration::from_millis(0)).as_millis() % MILLIS_BUDGET_PER_FRAME.as_millis();
                    let mut time_until_next_deadline = Duration::from_millis((MILLIS_BUDGET_PER_FRAME.as_millis() - millis_in) as u64);
                    if time_until_next_deadline < Duration::from_millis(2) {
                        log::debug!("Dropping a frame as we're close to deadline");
                        time_until_next_deadline = time_until_next_deadline + MILLIS_BUDGET_PER_FRAME;
                    }
                    let next_deadline: Instant = 
                        now
                            .checked_add(time_until_next_deadline)
                            .expect("We have reached the end of time.");
                    
                    log::debug!("Set next deadline: {:?} ({}ms)", next_deadline, time_until_next_deadline.as_millis());
                    deadline = Some(next_deadline);
                },
                recv(time_until_deadline.map(|d| after(d)).unwrap_or(never())) -> _timeout => {
                    log::debug!("Hit deadline for render");

                    if let Some(s) = last_state.take() {
                        display.update(s);
                    } else {
                        log::debug!("Reached render deadline but no state waiting");
                    }
                    deadline = None;
                }
            }
        }
    }).expect("Failed spawning input listener thread")
}

impl TerminalDisplay {
    fn update(&mut self, state: StateSnapshot) {
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

                    let escaped = 
                        hlstate
                            .as_ref()
                            .and_then(|hls| hls.highlighted_line(&line))
                            .unwrap_or(&txt);

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
