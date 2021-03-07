use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::{ops::Deref, usize};

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

pub struct LineView<'a> {
    line: &'a Line,
    rev: Rev,
}

impl<'a> LineView<'a> {
    pub fn content_str(&self) -> String {
        self.line.content.iter().collect::<String>()
    }
}

impl<'a> Deref for LineView<'a> {
    type Target = Line;

    fn deref(&self) -> &Line {
        self.line
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
        Some(LineView { rev, line })
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

// pub struct Line {
//     content: Vec<char>,
//     annotations: typedstore::TypedStore,
// }
