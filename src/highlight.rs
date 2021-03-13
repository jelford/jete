use std::{thread, time::Duration};



use crossbeam::channel;
use syntect::{highlighting::{Highlighter, ThemeSet}, parsing::{ParseState, ScopeStackOp, SyntaxSet}};

use crate::{pubsub::{self}, text::LineView};
use crate::state;
use crate::text::{Rev};

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



                // let escaped = as_24_bit_terminal_escaped(&hl_line.highlight_ranges[..], false);
                // return Some(escaped);
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
    highlight_ranges: Vec<(usize, ScopeStackOp)>,
    highlighted_rev: Rev,
}

impl Default for HighlightState {
    fn default() -> Self {
        HighlightState {
            highlighted_lines: Vec::new(),
        }
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
        let theme = &theme_set.themes["base16-ocean.dark"];
        let syntax = syntax_set.find_syntax_by_extension("rs").unwrap();

        let _highlighter = Highlighter::new(theme);
        
        log::debug!("setting up highlight thread");
        
        while let Ok(text) = r.recv() {
            log::debug!("Beginning highlight pass");
            
            let mut parse_state = ParseState::new(syntax);
            let mut highlighted_lines = Vec::with_capacity(text.line_count());

            let mut new_state = HighlightState::default();

            for line in text.iter_lines() {
                let line_text = line.content_str();
                let ranges = parse_state.parse_line(&line_text, &syntax_set);
                // let ranges =  highlight.highlight(&line_text, &syntax_set);
                highlighted_lines.push(HighlightedLine {
                    highlight_ranges: ranges,
                    highlighted_rev: line.rev(),
                });
            }

            new_state.highlighted_lines.append(&mut highlighted_lines);


            hub.send(HighlightState::topic(), new_state).unwrap();
            // let ranges = highlight.highlight(&txt, &ps);
            log::debug!("Highlight pass finished");
        }
    }).expect("Initializing highlighter");

    ready_receive.recv_timeout(Duration::from_millis(10)).expect("Unable to initialize highlighter");
}