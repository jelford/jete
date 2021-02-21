

use io::{BufRead, BufWriter, Seek, SeekFrom, Write};
use std::fs::File;

use crate::userinput::{Event, Key};

#[derive(Clone)]
pub struct Line {
    content: String,
}

impl Line {
    fn empty() -> Self {
        Line { content: String::new() }
    }

    fn from(existing: String) -> Self {
        Line { content: existing }
    }
}

pub struct CursorPos {
    pub line_number: usize,
    pub colmun: usize,
}

pub struct State {
    cursor_pos: CursorPos,
    lines: Vec<Line>,
    status_text: String,
    mode: Mode,
    command_line: String,
    file: Option<File>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Insert,
    Normal,
    Command,
}

pub enum EditorAction {
    Quit,
    None,
}

impl <'a> State {
    pub fn dispatch(&'a mut self, e: Event) -> EditorAction {
        match self.mode {
            Mode::Insert => {
                match e {
                    Event::Key(k) => {
                        match k {
                            Key::Esc => self.shift_mode(Mode::Normal),
                            Key::Char(c) => self.insert(c),
                            _ => {}
                        }
                    },
                    
                    _ => {}
                }
            },
            Mode::Command => {
                match e {
                    Event::Key(k) => {
                        match k {
                            Key::Esc => self.shift_mode(Mode::Normal),
                            Key::Char('\n') => {return self.commit_command()},
                            Key::Char(k) => self.insert(k),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Mode::Normal => {
                match e {
                    Event::Key(k) => {
                        match k {
                            Key::Char('u') => self.move_cursor((-1, 0)),
                            Key::Char('o') => self.move_cursor((0, 1)),
                            Key::Char('e') => self.move_cursor((1, 0)),
                            Key::Char('n') => self.move_cursor((0, -1)),
                            Key::Char(':') => self.shift_mode(Mode::Command),
                            Key::Char('i') => self.shift_mode(Mode::Insert),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        };

        EditorAction::None
    }


    pub fn insert(&mut self, c: char) {
        match self.mode {
            Mode::Insert => {
                let cur_ln = self.cursor_pos.line_number;
                let cur_col = self.cursor_pos.colmun;

                while self.lines.len() <= cur_ln {
                    self.lines.push(Line::empty());
                }

                let l = &mut self.lines[cur_ln];

                while l.content.len() < cur_col {
                    l.content.push(' ');
                }

                assert!(cur_col <= l.content.len());
                
                if c == '\n' {
                    let rest_of_line = l.content.split_off(cur_col);
                    self.lines.insert(cur_ln + 1, Line::from(rest_of_line));
                    self.cursor_pos.line_number += 1;
                    self.cursor_pos.colmun = 0;
                } else {
                    l.content.insert(cur_col.min(l.content.len()), c);
                    self.cursor_pos.colmun += 1;
                }

                self.status_text = format!("inserted char: {}", if c != '\n' { c } else { '\0' });
            }
            Mode::Command => {
                if c == '\n' {
                    
                } else {
                    self.command_line.push(c);
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
        if let Some(ref mut f) = self.file {
            f.seek(SeekFrom::Start(0)).expect("seeking to start of file");
            let num_lines = self.lines.len();
            let mut writer = BufWriter::new(f);
            for (i, l) in self.lines.iter().enumerate() {
                let _ = writer.write_all(l.content.as_bytes());
                if i+1 < num_lines {
                    let _ = writer.write(b"\n");
                }
            }
        }
    }

    pub fn line_text(&self, line_number: usize) -> Option<&str> {
        Some(&self.lines.get(line_number)?.content)
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
    }

    pub fn move_cursor(&mut self, direction: (i8, i8)) {

        let ln_inc = direction.0;
        let col_inc = direction.1;

        self.cursor_pos.line_number = if !ln_inc.is_negative() {
            self.cursor_pos.line_number.saturating_add(ln_inc as usize)
        } else {
            self.cursor_pos.line_number.saturating_sub(ln_inc.abs() as usize)
        }.min(self.lines.len().max(1)-1);

        self.cursor_pos.colmun = if !col_inc.is_negative() {
            self.cursor_pos.colmun.saturating_add(col_inc as usize)
        } else {
            self.cursor_pos.colmun.saturating_sub(col_inc.abs() as usize)
        }.min(self.line_text(self.cursor_pos.line_number).map(|t| t.len()).unwrap_or(0));
    }

    pub fn cursor_pos(&self) -> &CursorPos {
        &self.cursor_pos
    }

    pub fn command_line(&self) -> &str {
        &self.command_line
    }
}

pub fn empty<'a>() -> State {
    State{
        cursor_pos: CursorPos {
            line_number: 0,
            colmun: 0,
        },
        lines: Vec::new(),
        status_text: String::new(),
        mode: Mode::Normal,
        command_line: String::new(),
        file: None,
    }
}

use std::ffi::OsStr;
use std::fs::{OpenOptions};
use std::io::{self, BufReader};

pub fn from_file(fname: &OsStr) -> io::Result<State> {
    println!("opening {:?}", fname);

    let f = 
        OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(fname)?;
    let reader = BufReader::new(f.try_clone().expect("Unable to clone file handle for reading"));
    let mut lines = Vec::new();

    for l in reader.lines() {
        let l = l?;
        lines.push(Line::from(l));
    }

    Ok(State{
        cursor_pos: CursorPos {
            line_number: 0,
            colmun: 0,
        },
        lines,
        status_text: String::new(),
        mode: Mode::Normal,
        command_line: String::new(),
        file: Some(f),
    })
}