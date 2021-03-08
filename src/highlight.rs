use std::{collections::BTreeMap, thread, time::Duration};



use crossbeam::channel;
use syntect::{easy::HighlightLines, highlighting::{Theme, ThemeSet}, parsing::{SyntaxReference, SyntaxSet}, util::as_24_bit_terminal_escaped};

use crate::{pubsub::{self, Hub}, text::LineView};
use crate::state;
use crate::text::{self, Rev};

#[derive(Debug, Clone)]
pub struct HighlightState {
    highlighted_lines: Vec<HighlightedLine>,
}

impl HighlightState {
    pub fn highlighted_line(&self, line: &LineView) -> Option<String> {
        let ln = line.line_number();
        if let Some(hl_line) = self.highlighted_lines.get(ln) {
            let rev = line.rev();
            if hl_line.highlighted_rev >= rev {
                return Some(hl_line.escape_sequence.clone());
            }
        }

        None
    }

    pub fn topic() -> pubsub::TopicId<HighlightState> {
        pubsub::type_topic::<HighlightState>()
    }
}

#[derive(Debug, Clone)]
struct HighlightedLine {
    escape_sequence: String,
    highlighted_rev: Rev,
}

impl Default for HighlightState {
    fn default() -> Self {
        HighlightState {
            highlighted_lines: Vec::new(),
        }
    }
}

struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    hub: Hub,
}

impl Highlighter {
    fn highlight(&mut self, text: text::Text) {
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let syntax = self.syntax_set.find_syntax_by_extension("rs").unwrap();
        let mut highlight = HighlightLines::new(&syntax, theme);

        let mut highlighted_lines = Vec::with_capacity(text.line_count());

        let mut new_state = HighlightState::default();

        for line in text.iter_lines() {
            let line_text = line.content_str();
            let ranges =  highlight.highlight(&line_text, &self.syntax_set);
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            highlighted_lines.push(HighlightedLine {
                escape_sequence: escaped,
                highlighted_rev: line.rev(),
            });
        }

        new_state.highlighted_lines.append(&mut highlighted_lines);


        self.hub.send(HighlightState::topic(), new_state).unwrap();
    }
}

pub fn spawn_highlighter(mut hub: pubsub::Hub) {

    // coordinate our interaction with the pubsub system; need to be ready and listening
    // for messages before they are sent.
    let (ready_send, ready_receive) = channel::bounded::<()>(0);

    thread::Builder::new().name("highlighter".into()).spawn(move || {
        let r = hub.get_receiver(state::text_update_topic());
        ready_send.send(()).unwrap();


        let syntax_set = SyntaxSet::load_defaults_nonewlines();
        let theme_set = ThemeSet::load_defaults();

        let mut highlighter = Highlighter {
            syntax_set,
            theme_set,
            hub
        };
        
        log::debug!("setting up highlight thread");
        
        while let Ok(t) = r.recv() {
            log::debug!("Beginning highlight pass");
            highlighter.highlight(t);
            // let ranges = highlight.highlight(&txt, &ps);
            log::debug!("Highlight pass finished");
        }
    }).expect("Initializing highlighter");

    ready_receive.recv_timeout(Duration::from_millis(10)).expect("Unable to initialize highlighter");
}