//! Which entries of an event batch a later entry of the same batch has overtaken.
//!
//! Free of every X type so that it, and its test, compile on any host: see
//! `display/geometry.rs`.

use std::{collections::HashSet, hash::Hash};

/// For each entry, whether a later entry carries the same key.
///
/// An entry with no key is never superseded. Of the entries sharing a key only the last one is
/// current: the batch was read from the server in one go, so the earlier ones describe a state
/// that was already replaced when the first of them was looked at.
pub(crate) fn superseded<K: Eq + Hash>(keys: &[Option<K>]) -> Vec<bool> {
    let mut seen = HashSet::new();
    let mut marks = vec![false; keys.len()];
    for (index, key) in keys.iter().enumerate().rev() {
        if let Some(key) = key {
            marks[index] = !seen.insert(key);
        }
    }
    marks
}

#[cfg(test)]
mod tests;
