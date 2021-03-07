use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::{ops::Deref, usize};

use crossbeam::channel::Iter;

/// Provides a way to decide
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rev {
    rev: u64,
}

impl Default for Rev {
    fn default() -> Self {
        Rev { rev: 0 }
    }
}

unsafe impl Sync for Rev {}
unsafe impl Send for Rev {}

impl Rev {
    fn bump(&mut self) -> Self {
        self.rev += 1;
        *self
    }
}

pub struct Text {
    rev: Rev,
    revs: BTreeMap<usize, Rev>,
    lines: Vec<Line>,
}

#[derive(Debug)]
pub struct Line {
    content: Vec<char>,
}

impl<S> From<S> for Line
where
    S: Into<String>,
{
    fn from(s: S) -> Self {
        let s = s.into();
        Line {
            content: s.chars().collect(),
        }
    }
}

impl Line {
    pub fn from_chars(chars: Vec<char>) -> Self {
        Line { content: chars }
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

    pub fn extend_line(&mut self, mut other: Line) {
        assert!(!other.content.contains(&'\n'));
        self.content_mut().append(other.content_mut())
    }
}

#[derive(Debug)]
pub struct LineView<'a> {
    line: &'a Line,
    rev: Rev,
    line_number: usize,
}

impl<'a> LineView<'a> {
    pub fn content_str(&self) -> String {
        self.line.content.iter().collect::<String>()
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }
}

impl<'a> Deref for LineView<'a> {
    type Target = Line;

    fn deref(&self) -> &Line {
        self.line
    }
}


pub struct LineViewIterator<'a> {
    revs: &'a BTreeMap<usize, Rev>,
    lines: &'a [Line],
    idx: usize,
    starting_line_number: usize,
}


impl<'a> Iterator for LineViewIterator<'a> {
    type Item = LineView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.lines.len() {
            None
        } else {
            let line = &self.lines[self.idx];
            let rev = self.revs.range(..=self.idx).next_back().map(|(_, r)| *r).unwrap_or(Rev::default());
            let line_number = self.starting_line_number + self.idx;
            let ret = LineView {
                line, rev, line_number,
            };
            self.idx += 1;
            Some(ret)
        }
    }
}


impl Text {
    pub fn new() -> Self {
        Text {
            rev: Rev::default(),
            revs: BTreeMap::new(),
            lines: Vec::new(),
        }
    }

    pub fn from(lines: &[String]) -> Self {
        let mut text = Text {
            rev: Rev::default(),
            revs: BTreeMap::new(),
            lines: Vec::with_capacity(lines.len()),
        };

        for l in lines {
            text.lines.push(Line::from(l));
        }

        text
    }

    pub fn line(&self, ln_number: usize) -> Option<LineView> {
        let line = self.lines.get(ln_number)?;
        let rev = self
            .revs
            .range(..=ln_number)
            .next_back()
            .map(|(_, r)| *r)
            .unwrap_or_default();
        Some(LineView { rev, line , line_number: ln_number})
    }

    pub fn line_mut(&mut self, ln_number: usize) -> Option<&mut Line> {
        self.line_changed(ln_number);
        self.lines.get_mut(ln_number)
    }

    pub fn remove_line(&mut self, ln_number: usize) -> Option<Line> {
        if self.lines.len() <= ln_number {
            return None;
        }

        self.line_changed(ln_number);
        Some(self.lines.remove(ln_number))
    }

    pub fn insert_line<S>(&mut self, ln_number: usize, s: S)
    where
        S: Into<Line>,
    {
        self.line_changed(ln_number);
        self.lines.insert(ln_number, s.into());
    }

    pub fn insert_line_from_chars(&mut self, ln_number: usize, chars: Vec<char>) {
        self.line_changed(ln_number);
        self.lines.insert(ln_number, Line::from_chars(chars))
    }

    pub fn iter_lines<'a>(&'a self) -> impl Iterator<Item = LineView<'a>> {
        (0..self.lines.len()).map(move |i| self.line(i).unwrap())
    }

    pub fn iter_line_range<'a>(&'a self, start: usize, end: usize) -> LineViewIterator<'a> {

        let lines = self.lines.get(start.max(0)..end.min(self.lines.len()));
        
        LineViewIterator {
            revs: &self.revs,
            lines: lines.unwrap_or(&[]),
            idx: 0,
            starting_line_number: start,
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn line_changed(&mut self, ln_number: usize) {
        self.revs.insert(ln_number, self.rev.bump());
        let _ = self.revs.split_off(&(ln_number + 1));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_extract_entered_lines() {
        let mut t = Text::new();
        t.insert_line(0, "hello");
        t.insert_line(1, "world");


        let l = t.line(0).expect("inserted line not present");
        assert_eq!(l.content_str(), "hello");


        let l = t.line(1).expect("inserted line not present");
        assert_eq!(l.content_str(), "world");
    }

    
    #[test]
    fn iterate_over_contained_range() {
        let mut t = Text::new();
        t.insert_line(0, "hello");
        t.insert_line(1, "world");
        t.insert_line(2, "how");
        t.insert_line(3, "are");
        t.insert_line(4, "you");

        let mut it = t.iter_line_range(0, 5);

        assert_eq!(it.next().map(|lv| lv.content_str()), Some("hello".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("world".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("how".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("are".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("you".to_string()));
        assert!(it.next().is_none());
        
        let mut it = t.iter_line_range(0, 2);
        
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("hello".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("world".to_string()));
        assert!(it.next().is_none());


        let mut it = t.iter_line_range(3, 7);
        
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("are".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str()), Some("you".to_string()));
        assert!(it.next().is_none());
    }


    use std::collections::BTreeMap;

    #[test]
    fn slice_behaviour() {
        let mut v = vec!['a', 'b', 'c'];

        let mut cnt = 0;
        for _ in &v[0..] {
            cnt += 1;
        }
        assert_eq!(cnt, 3);

        let mut cnt = 0;
        for _ in &v[2..] {
            cnt += 1;
        }
        assert_eq!(cnt, 1);

        let mut cnt = 0;
        for _ in &v[3..] {
            cnt += 1;
        }
        assert_eq!(cnt, 0);

        let mut cnt = 0;
        if let Some(rng) = v.get_mut(3..) {
            for _ in rng {
                cnt += 1;
            }
        }
        assert_eq!(cnt, 0);
    }

    #[test]
    fn btree_behaviour() {
        let mut bt = BTreeMap::new();
        bt.insert(0, 'a');
        bt.insert(5, 'b');
        bt.insert(20, 'c');

        bt.split_off(&25);
        let v: Vec<char> = bt.values().cloned().collect();
        assert_eq!(v, ['a', 'b', 'c']);

        bt.split_off(&15);

        let v: Vec<char> = bt.values().cloned().collect();
        assert_eq!(v, ['a', 'b'])
    }
}
