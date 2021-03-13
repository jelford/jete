use std::{collections::BTreeMap};
use std::{ops::Deref, usize};

use std::sync::Arc;

use lazy_static::lazy_static;


lazy_static! {
    static ref EMPTY_STRING: Arc<String> = Arc::new(String::new());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rev {
    rev: u64,
}

impl From<u64> for Rev {
    fn from(v: u64) -> Self {
        Rev { rev: v }
    }
}

impl Default for Rev {
    fn default() -> Self {
        Rev { rev: 0 }
    }
}

unsafe impl Send for Rev {}

impl Rev {
    fn bump(mut self) -> Self {
        self.rev += 1;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineId {
    id: u64,
}

impl Default for LineId {
    fn default() -> Self {
        LineId { id: 0 }
    }
}

impl LineId {
    fn bump(mut self) -> Self {
        self.id += 1;
        self
    }
}

unsafe impl Send for LineId {}

#[derive(Clone)]
pub struct Text {
    rev: Rev,
    next_line_id: LineId,
    revs_before: BTreeMap<usize, Rev>,
    lines: Vec<Line>,
}

pub struct LineContent {
    content: Vec<char>,
    content_string: Arc<String>,
}

#[derive(Debug, Clone)]
pub struct Line {
    id: LineId,
    rev: Rev,
    content: Vec<char>,
    content_string: Arc<String>,
}

impl<'a, S> From<S> for LineContent
where
    S: Into<String>,
{
    fn from(s: S) -> Self {
        let s = s.into();
        LineContent {
            content: s.chars().collect(),
            content_string: Arc::new(s),
        }
    }
}

impl Line {

    pub fn content_string(&self) -> Arc<String> {
        self.content_string.clone()
    }

    pub fn char_count(&self) -> usize {
        self.content.len()
    }

    pub fn remove_char(&mut self, index: usize) {
        self.content.remove(index);
        self.on_content_change();
    }

    pub fn insert(&mut self, index: usize, c: char) {
        self.content.insert(index, c);
        self.on_content_change();
    }

    pub fn split_off(&mut self, index: usize) -> Vec<char> {
        let result = self.content.split_off(index);
        self.on_content_change();
        result
    }


    pub fn extend_line(&mut self, mut other: Line) {
        assert!(!other.content.contains(&'\n'));
        self.content.append(&mut other.content);
        self.on_content_change();
    }

    fn on_content_change(&mut self) {
        let new_content_string = self.content.iter().collect();
        self.content_string = Arc::new(new_content_string);
    }
}

#[derive(Debug)]
pub struct LineView<'a> {
    line: &'a Line,
    max_rev_before: Rev,
    line_number: usize,
}

impl<'a> LineView<'a> {
    pub fn content_str(&self) -> Arc<String> {
        self.line.content_string.clone()
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }

    pub fn id(&self) -> LineId {
        self.line.id
    }

    pub fn rev(&self) -> Rev {
        self.line.rev
    }

    pub fn max_rev_before(&self) -> Rev {
        self.max_rev_before
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
                line, max_rev_before: rev, line_number,
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
            next_line_id: LineId::default(),
            revs_before: BTreeMap::new(),
            lines: Vec::new(),
        }
    }

    fn bump_line_id(&mut self) -> LineId {
        self.next_line_id = self.next_line_id.bump();
        self.next_line_id
    }

    fn bump_rev(&mut self) -> Rev {
        self.rev = self.rev.bump();
        self.rev
    }

    pub fn from(lines: &[String]) -> Self {
        let mut text = Text {
            rev: Rev::default(),
            next_line_id: LineId::default(),
            revs_before: BTreeMap::new(),
            lines: Vec::with_capacity(lines.len()),
        };

        for l in lines {
            let l = Line {
                id: text.bump_line_id(),
                rev: Rev::default(),
                content: l.chars().collect(),
                content_string: Arc::new(l.clone()),
            };

            text.lines.push(l);
        }

        text
    }

    pub fn line(&self, ln_number: usize) -> Option<LineView> {
        let line = self.lines.get(ln_number)?;
        let rev = self
            .revs_before
            .range(..=ln_number)
            .next_back()
            .map(|(_, r)| *r)
            .unwrap_or_default();
        Some(LineView { max_rev_before: rev, line , line_number: ln_number})
    }

    pub fn line_mut(&mut self, ln_number: usize) -> Option<&mut Line> {
        let rev = self.bump_rev();
        self.line_changed(ln_number);
        self.lines.get_mut(ln_number).map(move |mut ln| {
            ln.rev = rev; 
            ln
        })
    }

    pub fn line_mut_populate(&mut self, ln_number: usize) -> &mut Line {
        self.bump_rev();
        if self.line_count() > ln_number {
            self.line_changed(ln_number);
            &mut self.lines[ln_number]
        } else {
            let number_of_new_lines = ln_number - self.lines.len() + 1;
            self.lines.reserve(number_of_new_lines);
            for _ in 0..number_of_new_lines {
                let l = Line {
                    id: self.bump_line_id(),
                    rev: self.rev,
                    content: vec![],
                    content_string: EMPTY_STRING.clone(),
                };
                self.lines.push(l);
            }

            &mut self.lines[ln_number]
        }
    }

    pub fn remove_line(&mut self, ln_number: usize) -> Option<Line> {
        if self.lines.len() <= ln_number {
            return None;
        }
        self.bump_rev();
        self.line_changed(ln_number);
        Some(self.lines.remove(ln_number))
    }

    pub fn insert_line<S>(&mut self, ln_number: usize, s: S)
    where
        S: Into<LineContent>,
    {
        let rev = self.bump_rev();
        let lc : LineContent = s.into();
        let line = Line {
            id: self.bump_line_id(),
            rev,
            content: lc.content,
            content_string: lc.content_string
        };
        self.lines.insert(ln_number, line);
        self.line_changed(ln_number);
    }

    pub fn insert_line_from_chars(&mut self, ln_number: usize, chars: Vec<char>) {
        let rev = self.bump_rev();
        let line_id = self.bump_line_id();
        let content_str = Arc::new(chars.iter().collect());
        
        self.lines.insert(ln_number, Line {
            id: line_id,
            rev,
            content: chars,
            content_string: content_str,
        });

        self.line_changed(ln_number);
    }

    pub fn iter_lines<'a>(&'a self) -> impl Iterator<Item = LineView<'a>> {
        (0..self.lines.len()).map(move |i| self.line(i).unwrap())
    }

    pub fn iter_line_range<'a>(&'a self, start: usize, end: usize) -> LineViewIterator<'a> {

        let lines = self.lines.get(start.max(0)..end.min(self.lines.len()));
        
        LineViewIterator {
            revs: &self.revs_before,
            lines: lines.unwrap_or(&[]),
            idx: 0,
            starting_line_number: start,
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn line_changed(&mut self, ln_number: usize) {
        self.revs_before.insert(ln_number, self.rev);
        let _ = self.revs_before.split_off(&(ln_number + 1));
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
        assert_eq!(*l.content_str(), "hello");


        let l = t.line(1).expect("inserted line not present");
        assert_eq!(*l.content_str(), "world");
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

        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("hello".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("world".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("how".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("are".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("you".to_string()));
        assert!(it.next().is_none());
        
        let mut it = t.iter_line_range(0, 2);
        
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("hello".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("world".to_string()));
        assert!(it.next().is_none());


        let mut it = t.iter_line_range(3, 7);
        
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("are".to_string()));
        assert_eq!(it.next().map(|lv| lv.content_str().to_string()), Some("you".to_string()));
        assert!(it.next().is_none());
    }

    #[test]
    fn get_line_number_beyond_current_count_populates_empty() {
        let mut t = Text::new();
        let l = t.line_mut_populate(24);

        // newly "got" line is empty
        assert_eq!(&*l.content_string(), "");
        assert_eq!(l.char_count(), 0);
        l.insert(0, 'x');
        assert_eq!(&*l.content_string(), "x");
        assert_eq!(l.char_count(), 1);

        assert_eq!(t.line_count(), 25);
        
    }

    #[test]
    fn test_revisions_get_bumped() {
        let mut t = Text::new();
        t.insert_line(0, "hello world");
        t.insert_line(1, "lorem ipsum");
        t.insert_line(2, "dolor sit amet, consectetur adipiscing elit");
        t.insert_line(3, "Donec cursus malesuada dui eu sagittis");

        let l = t.line_mut(2).unwrap();
        l.insert(5, 'x');

        let mut max_so_far = Rev::default();
        for l in t.iter_lines() {
            assert!(l.rev() <= l.max_rev_before());
            assert!(l.max_rev_before() >= max_so_far);
            max_so_far = max_so_far.max(l.max_rev_before());
        }

        assert_eq!(t.line(0).unwrap().rev(), Rev::from(1));
        assert_eq!(t.line(1).unwrap().rev(), Rev::from(2));
        assert_eq!(t.line(2).unwrap().rev(), Rev::from(5));
        assert_eq!(t.line(3).unwrap().rev(), Rev::from(4));
        
        assert_eq!(t.line(0).unwrap().max_rev_before(), Rev::from(1));
        assert_eq!(t.line(1).unwrap().max_rev_before(), Rev::from(2));
        assert_eq!(t.line(2).unwrap().max_rev_before(), Rev::from(5));
        assert_eq!(t.line(3).unwrap().max_rev_before(), Rev::from(5));
        
        assert!(t.line(4).is_none());
        
    }
}