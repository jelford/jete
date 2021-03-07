use std::{usize};

pub struct Text {
    lines: Vec<Line>,
}

pub struct Line {
    content: Vec<char>
}

impl<S> From<S> for Line where S : Into<String> {
    fn from(s: S) -> Self {
        let s = s.into();
        Line {
            content: s.chars().collect()
        }
    }
}

impl Line {
    pub fn from_chars(chars: Vec<char>) -> Self {
        Line {
            content: chars
        }
    }

    pub fn char_count(&self) -> usize {
        self.content.len()
    }

    pub fn remove_char(&mut self, index: usize) {
        self.content.remove(index);
    }

    pub fn content_mut(&mut self) -> &mut Vec<char> {
        &mut self.content
    }

    pub fn content_str(&self) -> String {
        self.content.iter().collect::<String>()
    }

    pub fn extend_line(&mut self, mut other: Line) {
        assert!(!other.content.contains(&'\n'));
        self.content.append(&mut other.content)
    }
}

impl Text {
    pub fn new() -> Self {
        Text {
            lines: Vec::new(),
        }
    }

    pub fn from(lines: &[String]) -> Self {

        let mut text = Text {
            lines: Vec::with_capacity(lines.len())
        };

        for l in lines {
            text.lines.push(Line::from(l));
        }

        text
    }

    pub fn line(&self, ln_number: usize) -> Option<&Line> {
        self.lines.get(ln_number)
    }
    
    pub fn line_mut(&mut self, ln_number: usize) -> Option<&mut Line> {
        self.lines.get_mut(ln_number)
    }

    pub fn remove_line(&mut self, ln_number: usize) -> Line {
        self.lines.remove(ln_number)
    }

    fn push_line() {

    }

    pub fn insert_line<S>(&mut self, ln_number: usize, s: S) 
        where S : Into<Line> {
        
        self.lines.insert(ln_number, s.into());
    }

    pub fn insert_line_from_chars(&mut self, ln_number: usize, chars: Vec<char>) {
        self.lines.insert(ln_number, Line::from_chars(chars))
    }

    pub fn iter_lines(&self) -> impl Iterator<Item=&Line> {
        self.lines.iter()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}



// pub struct Line {
//     content: Vec<char>,
//     annotations: typedstore::TypedStore,
// }
