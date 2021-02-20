use std::io::Stderr;


pub struct Line {
    content: String,
}

impl Line {
    fn empty() -> Self {
        Line { content: String::new() }
    }
}

pub struct CursorPos {
    line_number: usize,
    colmun: usize,
}

pub struct State {
    cursor_pos: CursorPos,
    lines: Vec<Line>,
    status_text: String,
}

impl State {
    pub fn insert(&mut self, c: char) {
        while self.lines.len() <= self.cursor_pos.line_number {
            self.lines.push(Line::empty());
        }

        let l = &mut self.lines[self.cursor_pos.line_number];
        l.content.push(c);

        if c == '\n' {
            self.cursor_pos.line_number += 1;
        } else {
            self.cursor_pos.colmun += 1;
        }

        self.status_text = format!("inserted char: {}", if c != '\n' { c } else { '\0' });
    }

    pub fn line_text(&self, line_number: usize) -> Option<&str> {
        Some(&self.lines.get(line_number)?.content)
    }

    pub fn status_text(&self) -> &str {
        &self.status_text
    }
}

pub fn empty() -> State {
    State{
        cursor_pos: CursorPos {
            line_number: 0,
            colmun: 0,
        },
        lines: vec![],
        status_text: String::new(),
    }
}