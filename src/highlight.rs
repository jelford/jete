use std::{collections::{HashMap, HashSet, hash_map::DefaultHasher}, fmt::{Display, Formatter}, hash::{self, Hash, Hasher}, sync::{Condvar, Mutex, Arc}, thread, time::Duration};

use syntect::{highlighting::{ThemeSet}, parsing::{SyntaxSet}};

use crate::{pubsub::{self}, text::{LineView, TextView}};
use crate::state;
use crate::text::{Rev, LineId};

#[derive(Debug, Clone)]
pub struct HighlightState {
    highlighted_lines: HashMap<LineId, Arc<HighlightedLine>>,
}

impl HighlightState {
    pub fn highlighted_line(&self, line: &LineView) -> Option<Arc<HighlightedLine>> {
        let ln = line.id();
        if let Some(hl_line) = self.highlighted_lines.get(&ln) {
            let rev = line.rev();
            if hl_line.highlighted_line_rev >= rev {
                return Some(hl_line.clone());
            }
        }

        None
    }

    pub fn topic() -> pubsub::TopicId<HighlightState> {
        pubsub::typed_topic::<HighlightState>("highlight")
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum HighlightRev {
    Rev {id: u64},
    None,
}



impl HighlightRev {
    fn from(highlighter_output: &str, line_id: LineId) -> Self {
        let mut h = DefaultHasher::new();
        highlighter_output.hash(&mut h);
        line_id.hash(&mut h);
        HighlightRev::Rev {
            id: h.finish()
        }
    }
}

impl Display for HighlightRev {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        match self {
            HighlightRev::None => (-1).fmt(fmt),
            HighlightRev::Rev { id } => (id%100).fmt(fmt)
        }
    }
}

impl Default for HighlightRev {
    fn default() -> Self {
        HighlightRev::None
    }
}


#[derive(Debug)]
pub struct HighlightedLine {
    highlighted_text: Arc<String>,
    highlighted_line_rev: Rev,
    highlight_rev: HighlightRev,
}

impl HighlightedLine {
    pub fn highlighted_text(&self) -> Arc<String> {
        self.highlighted_text.clone()
    }

    pub fn rev(&self) -> HighlightRev {
        self.highlight_rev
    }
}

pub fn spawn_highlighter(mut hub: pubsub::Hub) {

    let text_receiver = hub.get_receiver(state::text_update_topic());
    let latest_state_sender: Arc<(Mutex<Option<TextView>>, Condvar)> = Arc::new((Mutex::new(None), Condvar::new()));
    let latest_state_consumer = latest_state_sender.clone();

    thread::Builder::new().name("highlight-coalescer".into()).spawn(move || {
        let (lock, cond) = &*latest_state_sender;

        for state in text_receiver {
            let mut state_holder = lock.lock().expect("publishing latest state");
            if state_holder.is_some() {
                log::debug!("skipping a state update...");
            }
            *state_holder = Some(state);
            cond.notify_one();
        }

    }).expect("spawning highlight thread");

    thread::Builder::new().name("highlighter".into()).spawn(move || {
       


        let syntax_set = SyntaxSet::load_defaults_nonewlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = &theme_set.themes["base16-ocean.dark"];
        let syntax = syntax_set.find_syntax_by_extension("rs").unwrap();

        log::debug!("setting up highlight thread");

        let mut prev_hl_state = HighlightState {
            highlighted_lines: HashMap::new()
        };

        loop {
            let (lock, cond) = &*latest_state_consumer;
            let text = {
                let mut new_state = lock.lock().expect("getting latest state");
                while new_state.is_none() {
                    new_state = cond.wait(new_state).expect("getting latest state");
                }
                new_state.take().unwrap()
            };


            log::debug!("Beginning highlight pass");
            
            let mut new_state = prev_hl_state.clone();

            let mut h = syntect::easy::HighlightLines::new(syntax, theme);

            let mut seen_lines = HashSet::with_capacity(prev_hl_state.highlighted_lines.len());

            for line in text.iter_lines() {
                let line_text = line.content_str();
                seen_lines.insert(line.id());
                let ranges = h.highlight(&line_text, &syntax_set);
                let escaped = syntect::util::as_24_bit_terminal_escaped(&ranges[..], false);
                let highlight_rev = HighlightRev::from(&escaped, line.id());

                new_state.highlighted_lines.insert(line.id(), Arc::new(HighlightedLine {
                    highlighted_text: Arc::new(escaped),
                    highlighted_line_rev: line.max_rev_before(),
                    highlight_rev,
                }));

                if line.line_number() > 0 && line.line_number() % 20 == 0 {
                    let _ = hub.send(HighlightState::topic(), new_state.clone());
                }
            }
            
            if let Err(_) = hub.send(HighlightState::topic(), new_state.clone()) {
                log::debug!("Nobody is listening for highlight updates");
            }
            
            log::debug!("Highlight pass finished");

            prev_hl_state = new_state;
            prev_hl_state.highlighted_lines.retain(|lid, _| seen_lines.contains(lid));
        }
        
    }).expect("Initializing highlighter");
}