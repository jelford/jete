use std::any::{Any, TypeId};
use std::sync::Arc;

use ahash::AHashMap;

#[derive(Clone)]
pub struct TypedStore {
    values: AHashMap<TypeId, Arc<dyn Any+Send>>,
}

impl TypedStore {
    pub fn new() -> Self {
        // This store will often be created with no elements
        TypedStore {
            values: AHashMap::with_capacity(0),
        }
    }

    pub fn set<T: 'static+Send>(&mut self, val: T) {
        let key = TypeId::of::<T>();
        let val = Arc::new(val);
        self.values.insert(key, val);
    }
    
    pub fn get<T: 'static+Send>(&self) -> Option<&T> {
        let key = TypeId::of::<T>();
        self.get_by_id(key)
    }

    pub fn get_by_id<T: 'static+Send>(&self, key: TypeId) -> Option<&T> {
        self.values.get(&key).map(|any| {
            any.downcast_ref::<T>()
                .expect("Internal error; type doesn't match TypeId::of::<type>()")
        })
    }
}

pub fn new_typedstore() -> TypedStore {
    TypedStore::new()
}

#[cfg(test)]
mod tests {
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

        assert_eq!(&A { value: 12 }, ts.get().expect("inserted value missing"));
        assert_ne!(&A { value: 13 }, ts.get().expect("inserted value missing"));

        assert_eq!(&B { value: 13 }, ts.get().expect("inserted value missing"));
        assert_ne!(&B { value: 12 }, ts.get().expect("inserted value missing"));
    }

    #[test]
    fn getting_missing_value_returns_none() {
        let mut ts = new_typedstore();

        assert_eq!(None, ts.get::<u8>());
        ts.set(1u8);
        assert_eq!(Some(&1u8), ts.get());
    }
}
