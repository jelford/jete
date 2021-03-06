use crossbeam::channel::{self, Receiver, Sender};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

#[derive(Clone)]
#[non_exhaustive]
pub struct TopicId<T> {
    id: TopicIdInternal,
    _type: PhantomData<T>,
}

impl<T> fmt::Debug for TopicId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TopicId").field("id", &self.id).finish()
    }
}

impl<T> fmt::Display for TopicId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

pub fn typed_topic<A: 'static>(name: &'static str) -> TopicId<A> {
    TopicId {
        id: TopicIdInternal::Type {
            name,
            tipe: TypeId::of::<A>(),
        },
        _type: PhantomData,
    }
}

impl fmt::Display for TopicIdInternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopicIdInternal::Type { name, .. } => name.fmt(f),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum TopicIdInternal {
    Type {
        name: &'static str,
        tipe: std::any::TypeId,
    },
}

struct Topic<T> {
    senders: Vec<Sender<T>>,
}

#[derive(Clone)]
pub struct Hub {
    internal: Arc<Mutex<HubInternal>>,
}

struct HubInternal {
    topics: HashMap<TopicIdInternal, Box<dyn Any + Send>>,
}

impl Hub {
    pub fn new() -> Self {
        Hub {
            internal: Arc::new(Mutex::new(HubInternal::new())),
        }
    }

    pub fn send<T: 'static + Clone + Send>(
        &mut self,
        topic: TopicId<T>,
        value: T,
    ) -> Result<(), ()> {
        let mut internal = self.internal.lock().unwrap();
        internal.send(topic, value)
    }

    pub fn get_receiver<T: 'static + Send>(&mut self, topic: TopicId<T>) -> Receiver<T> {
        let mut internal = self.internal.lock().unwrap();
        internal.get_receiver(topic)
    }
}

impl HubInternal {
    fn new() -> Self {
        HubInternal {
            topics: HashMap::new(),
        }
    }

    fn send<T: 'static + Clone + Send>(&mut self, topic: TopicId<T>, value: T) -> Result<(), ()> {
        log::debug!("Sending update on topic: {}", topic);
        let t = self.get_or_create_topic(&topic);

        let mut closed_channels = Vec::new();
        for (i, s) in t.senders.iter().enumerate() {
            let result = s.send(value.clone()).map_err(|_| ());
            if let Err(_) = result {
                closed_channels.push(i);
            }
        }

        if closed_channels.len() > 0 {
            log::debug!("Cleaning closed channels for topic: {}", topic);
        }
        for closed in closed_channels.iter().rev() {
            t.senders.swap_remove(*closed);
        }

        if t.senders.len() > 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    fn get_receiver<T: 'static + Send>(&mut self, topic: TopicId<T>) -> Receiver<T> {
        log::debug!("Giving out receiver for {}", topic);
        let t = self.get_or_create_topic(&topic);
        let (s, r) = channel::unbounded();
        t.senders.push(s);
        r
    }

    fn get_or_create_topic<T: 'static + Send>(&mut self, topic: &TopicId<T>) -> &mut Topic<T> {
        self.topics
            .entry(topic.id)
            .or_insert_with(|| {
                log::debug!("Setting up channel for {}", topic);
                let t: Topic<T> = Topic {
                    senders: Vec::new(),
                };
                Box::new(t)
            })
            .downcast_mut()
            .expect("Internal state inconsistent")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn can_receive_notification() {
        let mut h = Hub::new();
        let topic = typed_topic::<u8>("test");
        let receiver = h.get_receiver(topic.clone());

        h.send(topic, 5).unwrap();

        assert_eq!(receiver.recv().unwrap(), 5);
    }

    #[test]
    fn can_receive_from_cloned_hub() {
        let mut h1 = Hub::new();
        let topic = typed_topic::<u8>("test");
        let receiver = h1.get_receiver(topic.clone());

        let mut h2 = h1.clone();
        h2.send(topic, 5).unwrap();

        assert_eq!(receiver.recv().unwrap(), 5);
    }

    #[test]
    fn cross_threads() {
        let mut h1 = Hub::new();
        let mut h2 = h1.clone();

        let (sync_send, sync_receive) = channel::bounded(0);

        let t = std::thread::spawn(move || {
            let r = h2.get_receiver(typed_topic::<u8>("test"));
            sync_send
                .send(())
                .expect("Failed trying to signal to main thread that we're ready for assertions");
            let result = r.recv_timeout(Duration::from_millis(30));
            result.unwrap();
        });

        sync_receive
            .recv_timeout(Duration::from_millis(50))
            .expect("Never got the go-ahead from receiver");
        h1.send(typed_topic::<u8>("test"), 12)
            .expect("Sending failed - no receiver?");

        t.join().unwrap();

        h1.send(typed_topic::<u8>("test"), 13)
            .expect_err("Should fail now as no subscribers");
    }
}
