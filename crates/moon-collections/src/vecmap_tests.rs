//! Tests for the VecMap collection.
//!
//! This is in a sibling module so that the tests are guaranteed to only cover
//! states that can be created by the public API.

use crate::vecmap::*;

#[test]
fn test_entry_vacant_or_insert() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    let value = map.entry("a").or_insert(1);
    assert_eq!(*value, 1);
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&"a", &1)]);
}

#[test]
fn test_entry_occupied_or_insert_keeps_existing() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    map.entry("a").or_insert(1);
    let value = map.entry("a").or_insert(99);
    assert_eq!(*value, 1);
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&"a", &1)]);
}

#[test]
fn test_entry_or_insert_with() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    map.entry("a").or_insert_with(|| 42);
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&"a", &42)]);
}

#[test]
fn test_entry_or_insert_with_not_called_when_occupied() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    map.entry("a").or_insert(1);
    map.entry("a")
        .or_insert_with(|| panic!("should not be called"));
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&"a", &1)]);
}

#[test]
fn test_entry_or_insert_with_key() {
    let mut map: VecMap<&str, String> = VecMap::new();
    map.entry("hello").or_insert_with_key(|k| k.to_uppercase());
    assert_eq!(
        map.iter().collect::<Vec<_>>(),
        vec![(&"hello", &"HELLO".to_string())]
    );
}

#[test]
fn test_entry_or_insert_default() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    map.entry("a").or_insert_default();
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&"a", &0)]);
}

#[test]
fn test_entry_key() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    assert_eq!(*map.entry("a").key(), "a");
    map.entry("a").or_insert(1);
    assert_eq!(*map.entry("a").key(), "a");
}

#[test]
fn test_entry_mut_ref_can_be_updated() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    let value = map.entry("a").or_insert(0);
    *value = 5;
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&"a", &5)]);
}

#[test]
fn test_insertion_order_preserved() {
    let mut map: VecMap<&str, i32> = VecMap::new();
    map.entry("b").or_insert(2);
    map.entry("a").or_insert(1);
    map.entry("c").or_insert(3);
    assert_eq!(
        map.iter().collect::<Vec<_>>(),
        vec![(&"b", &2), (&"a", &1), (&"c", &3)]
    );
}

#[test]
fn test_multiple_entries_independent() {
    let mut map: VecMap<i32, i32> = VecMap::new();
    map.entry(1).or_insert(10);
    map.entry(2).or_insert(20);
    map.entry(3).or_insert(30);
    assert_eq!(map.iter().count(), 3);
    // Re-inserting does not duplicate keys
    map.entry(1).or_insert(99);
    assert_eq!(map.iter().count(), 3);
}

// entry_ref tests

use std::cell::Cell;
use std::rc::Rc;

#[derive(PartialEq, Eq)]
struct CountedKey {
    value: String,
    clone_count: Rc<Cell<usize>>,
}

impl Clone for CountedKey {
    fn clone(&self) -> Self {
        self.clone_count.set(self.clone_count.get() + 1);
        CountedKey {
            value: self.value.clone(),
            clone_count: self.clone_count.clone(),
        }
    }
}

#[test]
fn test_entry_ref_vacant_or_insert() {
    let mut map: VecMap<String, i32> = VecMap::new();
    let key = "a".to_string();
    let value = map.entry_ref(&key).or_insert(1);
    assert_eq!(*value, 1);
    assert_eq!(map.iter().count(), 1);
}

#[test]
fn test_entry_ref_occupied_or_insert_keeps_existing() {
    let mut map: VecMap<String, i32> = VecMap::new();
    map.entry_ref(&"a".to_string()).or_insert(1);
    let value = map.entry_ref(&"a".to_string()).or_insert(99);
    assert_eq!(*value, 1);
    assert_eq!(map.iter().count(), 1);
}

#[test]
fn test_entry_ref_key_not_cloned_when_occupied() {
    let clone_count = Rc::new(Cell::new(0));
    let key = CountedKey {
        value: "a".to_string(),
        clone_count: clone_count.clone(),
    };

    let mut map: VecMap<CountedKey, i32> = VecMap::new();
    map.entry_ref(&key).or_insert(1);
    let clones_after_insert = clone_count.get();

    // Looking up an existing key must not clone it.
    map.entry_ref(&key).or_insert(99);
    assert_eq!(clone_count.get(), clones_after_insert);
}

#[test]
fn test_entry_ref_key_cloned_exactly_once_on_vacant_insert() {
    let clone_count = Rc::new(Cell::new(0));
    let key = CountedKey {
        value: "a".to_string(),
        clone_count: clone_count.clone(),
    };

    let mut map: VecMap<CountedKey, i32> = VecMap::new();
    map.entry_ref(&key).or_insert(1);
    assert_eq!(clone_count.get(), 1);
}

#[test]
fn test_entry_ref_or_insert_with_key() {
    let mut map: VecMap<String, String> = VecMap::new();
    let key = "hello".to_string();
    map.entry_ref(&key).or_insert_with_key(|k| k.to_uppercase());
    assert_eq!(
        map.iter().collect::<Vec<_>>(),
        vec![(&"hello".to_string(), &"HELLO".to_string())]
    );
}

#[test]
fn test_entry_ref_or_insert_with_not_called_when_occupied() {
    let mut map: VecMap<String, i32> = VecMap::new();
    let key = "a".to_string();
    map.entry_ref(&key).or_insert(1);
    map.entry_ref(&key)
        .or_insert_with(|| panic!("should not be called"));
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&key, &1)]);
}

#[test]
fn test_entry_ref_or_insert_default() {
    let mut map: VecMap<String, i32> = VecMap::new();
    map.entry_ref(&"a".to_string()).or_insert_default();
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&"a".to_string(), &0)]);
}

#[test]
fn test_entry_ref_key() {
    let mut map: VecMap<String, i32> = VecMap::new();
    let key = "a".to_string();
    assert_eq!(*map.entry_ref(&key).key(), key);
    map.entry_ref(&key).or_insert(1);
    assert_eq!(*map.entry_ref(&key).key(), key);
}

#[test]
fn test_entry_ref_mut_ref_can_be_updated() {
    let mut map: VecMap<String, i32> = VecMap::new();
    let key = "a".to_string();
    let value = map.entry_ref(&key).or_insert(0);
    *value = 5;
    assert_eq!(map.iter().collect::<Vec<_>>(), vec![(&key, &5)]);
}

/// A key whose `Clone` panics, standing in for any fallible clone (an allocation, a refcount).
#[derive(PartialEq, Eq, Debug)]
struct PanickingCloneKey(u32);

impl Clone for PanickingCloneKey {
    fn clone(&self) -> Self {
        panic!("clone of key {} failed", self.0);
    }
}

/// Catches `EntryRef::or_insert_with_key` pushing the value before cloning the key: a clone that
/// panics then leaves one value with no key, so every key inserted afterwards reads the value
/// stored beside the one before it. Like `HashMap`, a panic in caller code must leave the map as
/// it was: after the failed insert, key 2 maps to "two", exactly as if the insert never ran.
#[test]
fn test_entry_ref_panicking_key_clone_leaves_map_unchanged() {
    let mut map: VecMap<PanickingCloneKey, &str> = VecMap::new();
    map.entry(PanickingCloneKey(1)).or_insert("one");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        map.entry_ref(&PanickingCloneKey(99)).or_insert("lost");
    }));
    assert!(result.is_err());

    map.entry(PanickingCloneKey(2)).or_insert("two");
    let pairs: Vec<_> = map.iter().map(|(key, value)| (key.0, *value)).collect();
    assert_eq!(pairs, vec![(1, "one"), (2, "two")]);
}
