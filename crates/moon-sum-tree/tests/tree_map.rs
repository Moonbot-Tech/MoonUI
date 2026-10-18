//! Integration tests for the public `TreeMap` and `TreeSet` API.

use std::collections::{BTreeMap, BTreeSet};

use moon_sum_tree::{TreeMap, TreeSet};

/// Catches `TreeMap::extend` handing a batch with a repeated key straight to
/// `SumTree::edit`, which buffers every insert of that key, so the map ends up
/// holding two entries under one key and `iter` yields both.
///
/// The expected contents come from `std::collections::BTreeMap::extend`, whose
/// documented rule matches `TreeMap::insert`: a later value for a key replaces
/// the earlier one.
#[test]
fn extend_keeps_one_entry_per_key_and_the_last_value_wins() {
    let existing = [("x", 0), ("y", 9)];
    let batch = [("x", 1), ("z", 3), ("x", 2), ("z", 4), ("x", 5)];

    let mut map = TreeMap::default();
    let mut expected = BTreeMap::new();
    for (key, value) in existing {
        map.insert(key, value);
        expected.insert(key, value);
    }
    map.extend(batch);
    expected.extend(batch);

    assert_eq!(
        map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        expected.into_iter().collect::<Vec<_>>()
    );
}

/// Catches `TreeSet::extend` inheriting the same duplicate-key batch bug, so a
/// set built from `[1, 1]` reports the element twice.
#[test]
fn set_extend_stores_a_repeated_element_once() {
    let items = [3, 1, 3, 2, 1];

    let mut set = TreeSet::default();
    set.extend(items);

    assert_eq!(
        set.iter().copied().collect::<Vec<_>>(),
        items
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    );
}
