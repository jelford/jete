use crate::{highlight::HighlightRev, pubsub, text::{LineId, Rev}, userinput::{self}};
use crate::state::{Mode, StateSnapshot, state_update_topic};
use crate::userinput::{Event};
use crate::highlight::HighlightState;
use std::{io::{stdin, stdout, Stdin, Stdout, Write}, time::{Instant, Duration}, usize};
use crossbeam::select;
use crossbeam::channel::{after, never};
use termion::{clear, color::{self, Bg}, cursor, input::{Events, TermRead}, raw::{IntoRawMode, RawTerminal}, screen};
use std::thread;
use bouncer::Bouncer;

const FRAME_BUDGET: Duration = Duration::from_millis(16);

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

    let mut last_displayed = Vec::with_capacity(termion::terminal_size().unwrap().1 as usize +1);

    (
        TerminalDisplay {
            top_line: 0,
            stdout,
            last_displayed,
        },
        TerminalInput {
            events: stdin.events(),
        },
    )
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
        let highlight_receiver = display_hub.get_receiver(crate::highlight::HighlightState::topic());
        let shutdown_receiver = display_hub.get_receiver(crate::editor::shutdown_event_topic());

        log::debug!("Initializing display thread");

        let mut last_state = StateForDisplay {
            editor_state: None,
            highlighter_state: None,
        };

        let mut render_start_deadline = 
            Bouncer::builder()
                .time_between_deadlines(FRAME_BUDGET)
                .skip_hot_deadline(Duration::from_millis(2))
                .build();
        
        loop {
            if render_start_deadline.expired() {
                log::debug!("Render start deadline hit - updating display");
                display.update(&last_state);
                render_start_deadline.clear();
            }

            let time_until_deadline = render_start_deadline.duration_until_deadline();

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
                    last_state.editor_state = Some(msg.unwrap());
                    render_start_deadline.mark();
                },
                recv(highlight_receiver) -> msg => {
                    match msg {
                        Err(e) => {
                            log::debug!("Error on highlight receiver: {}", e);
                        },
                        Ok(msg) => {
                            last_state.highlighter_state = Some(msg);
                            render_start_deadline.mark();
                        },
                    };
                },
                recv(time_until_deadline.map(|d| after(d)).unwrap_or(never())) -> _timeout => {}
            }
        }
    }).expect("Failed spawning input listener thread")
}

struct StateForDisplay {
    editor_state: Option<StateSnapshot>,
    highlighter_state: Option<HighlightState>,
}

#[derive(Clone)]
enum LineDisplayRevision {
    New,
    Previous {
        line_id: LineId,
        line_rev: Option<Rev>,
        hl_rev: Option<HighlightRev>,
        screen_dims: (u16, u16),
    }
}

impl LineDisplayRevision {
    fn from(line_id: LineId, line_rev: Rev, hl_rev: Option<HighlightRev>, screen_dims: (u16, u16)) -> Self {
        LineDisplayRevision::Previous {
            line_id, line_rev: Some(line_rev), hl_rev, screen_dims
        }
    }

    fn is_new(&self, previous: &LineDisplayRevision) -> bool {
        match (self, previous) {
            (Self::New, _) => true,
            (_, Self::New) => true,
            (Self::Previous { line_id: my_line_id, line_rev: my_line_rev, hl_rev: my_hl_rev, screen_dims: my_screen_dims },
            Self::Previous { line_id, line_rev, hl_rev, screen_dims }) => {
                my_line_id != line_id 
                || line_rev.is_none() || my_line_rev != line_rev 
                || hl_rev.is_none() || my_hl_rev != hl_rev 
                || my_screen_dims != screen_dims
                
            }
        }
    }
}

impl Default for LineDisplayRevision {
    fn default() -> Self {
        LineDisplayRevision::New
    }
}

pub struct TerminalDisplay {
    top_line: usize,
    stdout: RawTerminal<Stdout>,
    last_displayed: Vec<LineDisplayRevision>
}

impl TerminalDisplay {
    fn update(&mut self, state: &StateForDisplay) {
        log::debug!("Render start");
        let (w, h) = termion::terminal_size().expect("unable to check terminal dimensions");

        let lines_at_bottom = 2u16;
        let text_view_height = h - lines_at_bottom;

        self.last_displayed.resize(h as usize + 1, LineDisplayRevision::default());

        if let Some(editor_state) = &state.editor_state {

            let cursor_pos = editor_state.cursor_pos();

            self.top_line = cursor_pos
                .line_number
                .saturating_sub((text_view_height as usize).saturating_sub(1));

            let hlstate = &state.highlighter_state;
            let text = editor_state.text();

            let mut text_lines = text.iter_line_range(self.top_line, self.top_line.saturating_add(text_view_height as usize));
            let mut output_line = 1;

            while output_line <= text_view_height {
                match text_lines.next() {
                    Some(line) => {


                        let txt = line.content_str();

                        let (escaped, hl_rev) = match hlstate.as_ref() {
                            Some(hls) => {
                                match hls.highlighted_line(&line) {
                                    Some(hll) => (hll.highlighted_text(), Some(hll.rev())),
                                    None => (txt, None),
                                }
                            },
                            None => {
                                (txt, None)
                            }
                        };

                        let now_key = LineDisplayRevision::from(line.id(), line.rev(), hl_rev, (w, h));
                        let last_time = &self.last_displayed[output_line as usize];
                        let should_render = now_key.is_new(last_time);

                        if should_render {
                            self.stdout.write_fmt(format_args!(
                                "{}{}{}{:3}@{:2}/{:2}|{}",
                                cursor::Goto(1, output_line),
                                color::Fg(color::Reset),
                                clear::CurrentLine,
                                line.line_number(),
                                line.rev(),
                                hl_rev.unwrap_or(HighlightRev::default()),
                                &escaped
                            )).expect("Unable to write to main text area");

                            self.last_displayed[output_line as usize] = now_key;
                        } else {
                            self.stdout.write_fmt(format_args!(
                                "{}{}{}{}",
                                cursor::Goto(4, output_line),
                                color::Bg(color::Blue),
                                "@",
                                color::Bg(color::Reset)
                            )).expect("Unable to write to main text area");
                        }
                    },
                    None => { 
                        self.stdout.write_fmt(format_args!(
                            "{}{}{}{:2}|~",
                            color::Fg(color::Reset),
                            cursor::Goto(1, output_line),
                            clear::CurrentLine,
                            self.top_line.saturating_add(output_line as usize - 1)
                        )).expect("Unable to write to main text area");
                    }
                };
                output_line += 1;
            }


            self.stdout.write_fmt(format_args!(
                "{}{}{}{}",
                color::Fg(color::Reset),
                color::Bg(color::Reset),
                cursor::Goto(1, h - 1),
                clear::CurrentLine
            )).unwrap();

            if editor_state.mode() == &Mode::Command {
                let command_text = editor_state.command_line();
                let command_text_disp = &command_text[command_text.len().saturating_sub(w as usize)..];
                self.stdout.write_fmt(format_args!(
                    "{}{}:{}",
                    cursor::Goto(1, h),
                    clear::CurrentLine,
                    command_text_disp
                )).unwrap();
            } else {
                let status_text = editor_state.status_text();
                let status_text_disp = &status_text[..status_text.len().min(w as usize - 1)];
                self.stdout.write_fmt(format_args!(
                    "{}{}{}\t{:?}\t(l:{},c:{})",
                    cursor::Goto(1, h),
                    clear::CurrentLine,
                    status_text_disp,
                    editor_state.mode(),
                    cursor_pos.line_number,
                    cursor_pos.colmun
                )).unwrap();

                let display_cursor_ln =
                    (1 + (cursor_pos.line_number - self.top_line) as u16).clamp(1, text_view_height);
                let display_cursor_col = (1 + cursor_pos.colmun as u16 + 10).clamp(1, w);

                self.stdout.write_fmt(format_args!(
                    "{}",
                    cursor::Goto(display_cursor_col, display_cursor_ln)
                )).unwrap();
            }
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
        let _ = self.stdout.write_fmt(format_args!(
            "{}{}",
            cursor::Goto(1, 1),
            clear::AfterCursor
        ));
        let _ = self.stdout.flush();
    }
}
