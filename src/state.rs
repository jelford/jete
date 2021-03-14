use crate::{pubsub::{self, Hub}, text::{Text, TextView}};
use crate::userinput::{Event, Key};
use std::{ffi::OsStr};
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::{fs::File, usize};


pub fn text_update_topic() -> pubsub::TopicId<TextView> {
    pubsub::typed_topic("body-text")
}

pub fn state_update_topic() -> pubsub::TopicId<StateSnapshot> {
    pubsub::typed_topic("state")
}

#[derive(Clone)]
pub struct CursorPos {
    pub line_number: usize,
    pub colmun: usize,
}

#[derive(Clone)]
pub struct StateSnapshot {
    cursor_pos: CursorPos,
    text: TextView,
    status_text: String,
    mode: Mode,
    command_line: String,
}

impl StateSnapshot {
    pub fn cursor_pos(&self) -> &CursorPos {
        &self.cursor_pos
    }

    pub fn text(&self) -> &TextView {
        &self.text
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn command_line(&self) -> &str {
        &self.command_line
    }

    pub fn status_text(&self) -> &str {
        &self.status_text
    }
}

pub struct State {
    cursor_pos: CursorPos,
    text: Text,
    status_text: String,
    mode: Mode,
    command_line: String,
    file: Option<File>,
    pubsub: Hub,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mode {
    Insert,
    Normal,
    Command,
}

pub enum EditorAction {
    Quit,
    None,
}

#[derive(Debug, Clone)]
pub enum Command {
    ShiftMode(Mode),
    DeleteAtCursor,
    InsertAtCursor(char),
    CommitCommandline,
    MoveCursor {
        lines_down: isize,
        columns_right: isize,
    },
}

pub fn input_map(current_mode: &Mode, e: Event) -> Option<Command> {
    match current_mode {
        Mode::Insert => match e {
            Event::Key(k) => match k {
                Key::Esc => Some(Command::ShiftMode(Mode::Normal)),
                Key::Backspace => Some(Command::DeleteAtCursor),
                Key::Char(c) => Some(Command::InsertAtCursor(c)),
                _ => None,
            },

            _ => None,
        },
        Mode::Command => match e {
            Event::Key(k) => match k {
                Key::Esc => Some(Command::ShiftMode(Mode::Normal)),
                Key::Char('\n') => Some(Command::CommitCommandline),
                Key::Backspace => Some(Command::DeleteAtCursor),
                Key::Char(c) => Some(Command::InsertAtCursor(c)),
                _ => None,
            },
            _ => None,
        },
        Mode::Normal => match e {
            Event::Key(k) => match k {
                Key::Char('u') => Some(Command::MoveCursor {
                    lines_down: -1,
                    columns_right: 0,
                }),
                Key::Char('o') => Some(Command::MoveCursor {
                    lines_down: 0,
                    columns_right: 1,
                }),
                Key::Char('e') => Some(Command::MoveCursor {
                    lines_down: 1,
                    columns_right: 0,
                }),
                Key::Char('n') => Some(Command::MoveCursor {
                    lines_down: 0,
                    columns_right: -1,
                }),
                Key::Char(':') => Some(Command::ShiftMode(Mode::Command)),
                Key::Char('i') => Some(Command::ShiftMode(Mode::Insert)),
                _ => None,
            },
            _ => None,
        },
    }
}

impl<'a> State {
    pub fn dispatch(&'a mut self, c: Command) -> EditorAction {
        log::debug!("dispatching {:?} in mode {:?}", c, self.mode);

        if let Command::ShiftMode(m) = c {
            self.shift_mode(m);
            self.notify_change();
            return EditorAction::None;
        }

        match self.mode {
            Mode::Insert => match c {
                Command::DeleteAtCursor => self.delete(),
                Command::InsertAtCursor(c) => self.insert(c),
                _ => {}
            },
            Mode::Command => match c {
                Command::DeleteAtCursor => self.delete(),
                Command::InsertAtCursor(c) => self.insert(c),
                Command::CommitCommandline => return self.commit_command(),
                _ => {}
            },
            Mode::Normal => match c {
                Command::MoveCursor {
                    lines_down,
                    columns_right,
                } => self.move_cursor((lines_down, columns_right)),
                _ => {}
            },
        };

        EditorAction::None
    }

    fn notify_change(&mut self) {
        if let Err(_) = self.pubsub.send(state_update_topic(), StateSnapshot{
            cursor_pos: self.cursor_pos.clone(),
            text: self.text.view(),
            status_text: self.status_text.clone(),
            mode: self.mode.clone(),
            command_line: self.command_line.clone(),
        }) {
            log::debug!("State changed but nobody's listening");
        }
    }

    pub fn insert(&mut self, c: char) {
        match self.mode {
            Mode::Insert => {
                let cur_ln = self.cursor_pos.line_number;
                let cur_col = self.cursor_pos.colmun;

                let l = self.text.line_mut_populate(cur_ln);

                assert!(cur_col <= l.char_count());

                let cur_ln = if c == '\n' {
                    let rest_of_line = l.split_off(cur_col);
                    self.text.insert_line_from_chars(cur_ln + 1, rest_of_line);
                    self.cursor_pos.line_number += 1;
                    self.cursor_pos.colmun = 0;
                    cur_ln + 1
                } else {
                    l.insert(cur_col, c);
                    self.cursor_pos.colmun += 1;
                    cur_ln
                };

                self.status_text = format!(
                    "char: {} @ ({},{})",
                    if c != '\n' { c as u8 } else { 0 },
                    cur_ln,
                    cur_col
                );

                self.notify_text_change();
            }
            Mode::Command => {
                if c == '\n' {
                } else {
                    self.command_line.push(c);
                    self.notify_change();
                }
            }

            _ => {}
        }
    }

    fn commit_command(&'a mut self) -> EditorAction {
        let action = self.command_line.clone();
        self.shift_mode(Mode::Normal);
        if action == "q" {
            EditorAction::Quit
        } else if action == "w" {
            self.write();
            EditorAction::None
        } else {
            EditorAction::None
        }
    }

    fn write(&mut self) {
        if let Some(f) = self.file.as_mut() {
            f.seek(SeekFrom::Start(0))
                .expect("seeking to start of file");
            let num_lines = self.text.line_count();

            let mut writer = BufWriter::new(f);
            for (i, l) in self.text.iter_lines().enumerate() {
                let write_result = writer.write_all(l.content_str().as_bytes()).and_then({
                    |_| {
                        if i < num_lines {
                            writer.write(b"\n")
                        } else {
                            Ok(0)
                        }
                    }
                });

                if let Err(e) = write_result {
                    self.status_text.clear();
                    self.status_text
                        .push_str(&format!("Failed to save file: {}", e));
                    return;
                }
            }

            let f = writer.get_mut();
            let new_file_length = f
                .seek(SeekFrom::Current(0))
                .expect("Unable to determine length of file being written");
            f.set_len(new_file_length)
                .expect("Unable to truncate file after writing");
        }
    }

    fn delete(&mut self) {
        match self.mode {
            Mode::Insert => {
                let cur_col = self.cursor_pos.colmun;
                if cur_col > 0 {
                    let line = self.text.line_mut(self.cursor_pos.line_number);
                    if let Some(line) = line {
                        line.remove_char(cur_col - 1);
                        self.cursor_pos.colmun = self.cursor_pos.colmun.saturating_sub(1);
                    }
                } else {
                    let cur_row = self.cursor_pos.line_number;

                    if cur_row == 0 {
                        if self.text.line_count() == 1 && self.text.line(0).expect("0th line missing").char_count() == 0 {
                            self.text.remove_line(0);
                        }
                        return;
                    }

                    let end_of_prev_line = self
                        .text
                        .line(cur_row - 1)
                        .map(|l| l.char_count())
                        .unwrap_or(0);

                    {
                        let cur_line = self.text.remove_line(cur_row);
                        if let Some(cur_line) = cur_line {
                            let prev_row = self.text.line_mut(cur_row - 1);
                            if let Some(prev_row) = prev_row {
                                prev_row.extend_line(cur_line);
                            }
                        }
                    }

                    let new_row = cur_row - 1;
                    self.cursor_pos.line_number = new_row;
                    self.cursor_pos.colmun = end_of_prev_line;
                };

                self.notify_text_change();
            }
            Mode::Command => {
                if self.command_line.len() > 0 {
                    self.command_line.remove(self.command_line.len() - 1);
                } else {
                    self.shift_mode(Mode::Normal);
                }
            }
            _ => {}
        }
    }

    fn notify_text_change(&mut self) {
        if let Err(_) = self.pubsub.send(text_update_topic(), self.text.view()) {
            log::debug!("Text updated but nobody's listening");
        }
        self.notify_change();
    }

    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn shift_mode(&mut self, m: Mode) {
        self.mode = m;
        self.command_line.clear();
        self.notify_change();
    }

    pub fn move_cursor(&mut self, direction: (isize, isize)) {
        match direction {
            (0, 0) => {}
            (ln, 0) => {
                self.cursor_pos.line_number = if !ln.is_negative() {
                    self.cursor_pos.line_number.saturating_add(ln as usize)
                } else {
                    self.cursor_pos
                        .line_number
                        .saturating_sub(ln.saturating_abs() as usize)
                }
                .clamp(0, self.text.line_count().saturating_sub(1));

                let line = self.text.line(self.cursor_pos.line_number);
                self.cursor_pos.colmun = line
                    .map(|l| self.cursor_pos.colmun.clamp(0, l.char_count()))
                    .unwrap_or(0);
                
                self.notify_change();
            }
            (0, col) => {
                let line = self.text.line(self.cursor_pos.line_number);
                if let Some(line) = line {
                    if !col.is_negative() {
                        self.cursor_pos.colmun = self
                            .cursor_pos
                            .colmun
                            .saturating_add(col as usize)
                            .clamp(0, line.char_count());
                    } else {
                        self.cursor_pos.colmun = self
                            .cursor_pos
                            .colmun
                            .saturating_sub(col.abs() as usize)
                            .clamp(0, line.char_count());
                    }

                    self.notify_change();
                }
            }

            (row, col) => {
                self.move_cursor((row, 0));
                self.move_cursor((0, col));
            }
        };

        assert!(self.cursor_pos.line_number <= self.text.line_count());
        if self.cursor_pos.line_number < self.text.line_count() {
            let line = &self.text.line(self.cursor_pos.line_number).unwrap();
            assert!(self.cursor_pos.colmun <= line.char_count());
        }
    }

    pub fn cursor_pos(&self) -> &CursorPos {
        &self.cursor_pos
    }

    pub fn command_line(&self) -> &str {
        &self.command_line
    }

    pub fn text(&self) -> &Text {
        &self.text
    }
}

pub fn empty<'a>(pubsub: Hub) -> State {
    State {
        cursor_pos: CursorPos {
            line_number: 0,
            colmun: 0,
        },
        text: Text::new(),
        status_text: String::new(),
        mode: Mode::Normal,
        command_line: String::new(),
        file: None,
        pubsub
    }
}

pub fn from_file(fname: &OsStr, pubsub: Hub) -> io::Result<State> {
    println!("opening {:?}", fname);

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(fname)?;
    let reader = BufReader::new(f.try_clone()?);
    let mut lines = Vec::new();

    for l in reader.lines() {
        let l = l?;
        lines.push(l);
    }

    let mut result = State {
        cursor_pos: CursorPos {
            line_number: 0,
            colmun: 0,
        },
        text: Text::from(&lines),
        status_text: String::new(),
        mode: Mode::Normal,
        command_line: String::new(),
        file: Some(f),
        pubsub: pubsub,
    };

    result.notify_text_change();

    Ok(result)
}
