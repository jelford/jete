use std::{ffi::OsString, sync::atomic::{AtomicBool, Ordering}};
use std::sync::Arc;

use crate::{display::Display, highlight};
use crate::state::{input_map, self, EditorAction};
use crate::pubsub::{Hub, self};
use std::thread;
use crossbeam::{channel::select};
use termion::event::Event;

use crate::userinput::UserInputSource;

pub fn run<Disp: Display, Inputs: UserInputSource>(fname: Option<OsString>, mut d: Disp, mut i: Inputs) {

    let mut hub = Hub::new();
    
    
    highlight::spawn_highlighter(hub.clone());
    
    let input_topic = pubsub::typed_topic::<Event>("input");
    let inputs = hub.get_receiver(input_topic.clone());
    let syntax_updates = hub.get_receiver(highlight::HighlightState::topic());
    
    let finished = Arc::new(AtomicBool::new(false));

    let state_hub = hub.clone();
    let mut input_hub = hub.clone();
    let mut display_hub = hub.clone();
    
    let other_finished = finished.clone();
    
    thread::Builder::new().name("input".into()).spawn(move || {
        for e in i.events() {
            let send_result = input_hub.send(input_topic.clone(), e);
            if send_result.is_err() {
                // nobody is listening
                break;
            }
        }
    }).expect("Failed spawning input listener thread");
    
    thread::Builder::new().name("display".into()).spawn(move || {
        let state_updates = display_hub.get_receiver(state::state_update_topic());
        for state in state_updates {
            d.update(&state);
        }
    }).expect("Failed to start display thread");


    let result = thread::Builder::new().name("core".into()).spawn(move || {
        let mut state = match fname {
            None => state::empty(state_hub),
            Some(fname) => state::from_file(&fname, state_hub).expect("Unable to read file"),
        };

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
                        log::warn!("highlight pipe closed");
                        // not fatal
                    }
                }
            }
        }

        log::debug!("finishing main state thread");
        other_finished.store(true, Ordering::SeqCst);
    }).expect("Failed spawning core editor thread")
        .join();

    if let Err(e) = result {
        if let Ok(e) = e.downcast::<String>() {
            log::error!("Core thread panicked: {}", e);
        }
        panic!("Core thread panicked");
    }

    log::debug!("Shutting down");
}
