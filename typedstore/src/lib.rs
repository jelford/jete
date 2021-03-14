use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex};

use ahash::AHashMap;

#[derive(Clone)]
pub struct TypedStore {
    internal: Arc<Mutex<TypedStoreInternal>>,
}

struct TypedStoreInternal {
    values: AHashMap<TypeId, Arc<dyn Any+Send+Sync>>,
}

impl TypedStore {
    pub fn new() -> Self {
        // This store will often be created with no elements
        TypedStore {
            internal: Arc::new(Mutex::new(
                TypedStoreInternal {
                    values: AHashMap::with_capacity(0),
                })),
        }
    }

    pub fn set<T: 'static+Send+Sync>(&mut self, val: T) {
        let mut internal = self.internal.lock().unwrap();
        internal.set(val);
    }

    pub fn get<T: 'static+Send+Sync>(&self) -> Option<Arc<T>> {
        let internal = self.internal.lock().unwrap();
        internal.get()
    }
}

impl TypedStoreInternal {
    fn set<T: 'static+Send+Sync>(&mut self, val: T) {
        let key = TypeId::of::<T>();
        let val = Arc::new(val);
        self.values.insert(key, val);
    }
    
    fn get<T: 'static+Send+Sync>(&self) -> Option<Arc<T>> {
        let key = TypeId::of::<T>();
        self.get_by_id(key)
    }

    fn get_by_id<T: 'static+Send+Sync>(&self, key: TypeId) -> Option<Arc<T>> {
        self.values.get(&key).map(|any| {
            any.clone().downcast::<T>()
                .expect("Internal error; type doesn't match TypeId::of::<type>()")
        })
    }
}

pub fn new_typedstore() -> TypedStore {
    TypedStore::new()
}

#[cfg(test)]
mod tests {
    use std::{time::Duration, sync::mpsc, thread};

    use super::*;
    #[test]
    fn can_store_std_types() {
        let mut ts = new_typedstore();

        ts.set(12u8);
        ts.set(13u64);
        ts.set(8i8);
        ts.set("test str");

        assert_eq!(12u8, *ts.get().expect("inserted value missing"));
        assert_ne!(13u8, *ts.get().expect("inserted value missing"));
        assert_eq!(13u64, *ts.get().expect("inserted value missing"));
        assert_eq!(8i8, *ts.get().expect("inserted value missing"));
        assert_eq!(
            "test str",
            *ts.get::<&str>().expect("inserted value missing")
        )
    }

    #[test]
    fn can_store_custom_type() {
        #[derive(PartialEq, Eq, Debug)]
        struct A {
            value: i8,
        }

        #[derive(PartialEq, Eq, Debug)]

        struct B {
            value: u8,
        }

        let mut ts = new_typedstore();

        ts.set(A { value: 12 });
        ts.set(B { value: 13 });

        assert_eq!(A { value: 12 }, *ts.get().expect("inserted value missing"));
        assert_ne!(A { value: 13 }, *ts.get().expect("inserted value missing"));

        assert_eq!(B { value: 13 }, *ts.get().expect("inserted value missing"));
        assert_ne!(B { value: 12 }, *ts.get().expect("inserted value missing"));
    }

    #[test]
    fn getting_missing_value_returns_none() {
        let mut ts = new_typedstore();

        assert_eq!(None, ts.get::<u8>());
        ts.set(1u8);
        assert_eq!(Some(Arc::new(1u8)), ts.get());
    }

    #[test]
    fn can_be_send_across_threads() {
        let mut ts = new_typedstore();
        ts.set(12u8);

        let (sender, receiver) = mpsc::sync_channel(1);
        let (result_sender, result_receiver) = mpsc::sync_channel(1);

        sender.send(ts).expect("unable to send to background thread");

        thread::spawn(move || {
            let ts = receiver.recv().expect("Failed receiving sent typedstore on background thread");
            let result = *ts.get::<u8>().expect("should have value stored in it");
            result_sender.send(result).expect("failed sending result to main test thread");
        }).join().expect("background thread failed");

        let result = result_receiver.recv_timeout(Duration::from_millis(50)).expect("Never got result");
        assert_eq!(result, 12u8);

    }
}
