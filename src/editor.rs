use std::sync::Arc;
use std::{
    ffi::OsString,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::pubsub::{self, Hub};
use crate::state::{self, input_map, EditorAction};
use crate::terminal;
use crate::{
    highlight,
    pubsub::{typed_topic, TopicId},
};
use crossbeam::channel::select;
use std::thread;
use termion::event::Event;

pub fn shutdown_event_topic() -> TopicId<()> {
    typed_topic("shutdown")
}

pub fn run(fname: Option<OsString>) {
    let mut hub = Hub::new();

    highlight::spawn_highlighter(hub.clone());
    let terminal_thread = terminal::spawn_interface(hub.clone());

    let input_topic = pubsub::typed_topic::<Event>("input");
    let inputs = hub.get_receiver(input_topic.clone());

    let finished = Arc::new(AtomicBool::new(false));

    let state_hub = hub.clone();

    let other_finished = finished.clone();

    let result = thread::Builder::new()
        .name("core".into())
        .spawn(move || {
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
                }
            }

            log::debug!("finishing main state thread");
            other_finished.store(true, Ordering::SeqCst);
        })
        .expect("Failed spawning core editor thread")
        .join();

    let _ = hub.send(shutdown_event_topic(), ());

    terminal_thread
        .join()
        .expect("Unable to join terminal thread");

    if let Err(e) = result {
        if let Ok(e) = e.downcast::<String>() {
            log::error!("Core thread panicked: {}", e);
        }
        panic!("Core thread panicked");
    }

    log::debug!("Shutting down");
}
