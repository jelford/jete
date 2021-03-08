use std::{ffi::OsString, sync::atomic::{AtomicBool, Ordering}};
use std::sync::Arc;

use crate::{display::Display, highlight};
use crate::state::{input_map, self, EditorAction};
use crate::pubsub::{Hub, self};
use std::thread;
use crossbeam::channel::select;
use termion::event::Event;

use crate::userinput::UserInputSource;

pub fn run<Disp: Display+'static, Inputs: UserInputSource>(fname: Option<OsString>, mut d: Disp, mut i: Inputs) {

    let mut hub = Hub::new();
    
    
    highlight::spawn_highlighter(hub.clone());
    
    let input_topic = pubsub::type_topic::<Event>();
    let inputs = hub.get_receiver(input_topic.clone());
    let syntax_updates = hub.get_receiver(highlight::HighlightState::topic());
    
    let finished = Arc::new(AtomicBool::new(false));

    let state_hub = hub.clone();

    let other_finished = finished.clone();
    thread::spawn(move || {
        let mut state = match fname {
            None => state::empty(state_hub),
            Some(fname) => state::from_file(&fname, state_hub).expect("Unable to read file"),
        };

        d.update(&state);

        loop {
            select! {
                recv(inputs) -> input => {
                    if let Ok(e) = input {
                        if let Some(command) = input_map(state.mode(), e) {
                            let editor_action = state.dispatch(command);
                            match editor_action {
                                EditorAction::Quit => break,
                                _ => {}
                            }
                        }
                    } else {
                        log::debug!("command pipe closed");
                        break;
                    }
                }
                recv(syntax_updates) -> syntax => {
                    if let Ok(highlight_state) = syntax {
                        state.dispatch_annotation_update(highlight_state);
                    } else {
                        log::debug!("highlight pipe closed");
                        // not fatal
                    }
                }
            }

            d.update(&state);
        }
        d.update(&state);

        log::debug!("finishing main state thread");
        other_finished.store(true, Ordering::SeqCst);
    });

    for e in i.events() {
        hub.send(input_topic.clone(), e).expect("input feed pipe");   
        if finished.load(Ordering::SeqCst) {
            break;
        } else {
            log::debug!("not finished...");
        }
    }

    log::debug!("Shutting down");
}
