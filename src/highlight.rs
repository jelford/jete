use std::{collections::BTreeMap, thread, time::Duration};



use crossbeam::channel;
use syntect::{highlighting::{ThemeSet}, parsing::{SyntaxSet}};

use crate::{pubsub::{self}, text::LineView};
use crate::state;
use crate::text::{Rev, LineId};

#[derive(Debug, Clone)]
pub struct HighlightState {
    highlighted_lines: BTreeMap<LineId, HighlightedLine>,
}

impl HighlightState {
    pub fn highlighted_line(&self, line: &LineView) -> Option<&str> {
        let ln = line.id();
        if let Some(hl_line) = self.highlighted_lines.get(&ln) {
            let rev = line.rev();
            if hl_line.highlighted_rev >= rev {
                return Some(&hl_line.highlighted_text)
            }
        }

        None
    }

    pub fn topic() -> pubsub::TopicId<HighlightState> {
        pubsub::typed_topic::<HighlightState>("highlight")
    }
}

#[derive(Debug, Clone)]
struct HighlightedLine {
    highlighted_text: String,
    highlighted_rev: Rev,
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

        log::debug!("setting up highlight thread");
        
        while let Ok(text) = r.recv() {
            log::debug!("Beginning highlight pass");
            
            let mut highlighted_lines = BTreeMap::new();
            

            let mut h = syntect::easy::HighlightLines::new(syntax, theme);

            for line in text.iter_lines() {
                let line_text = line.content_str();
                
                
                let ranges = h.highlight(&line_text, &syntax_set);
                let escaped = syntect::util::as_24_bit_terminal_escaped(&ranges[..], false);

                highlighted_lines.insert(line.id(), HighlightedLine {
                    highlighted_text: escaped,
                    highlighted_rev: line.rev(),
                });
            }
            let new_state = HighlightState {
                highlighted_lines: highlighted_lines,
            };

            if let Err(_) = hub.send(HighlightState::topic(), new_state) {
                log::debug!("Nobody is listening for highlight updates");
            }
            
            log::debug!("Highlight pass finished");
        }
    }).expect("Initializing highlighter");

    ready_receive.recv_timeout(Duration::from_millis(10)).expect("Unable to initialize highlighter");
}