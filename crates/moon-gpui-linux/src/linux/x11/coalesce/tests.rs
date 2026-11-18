use super::superseded;

#[test]
fn only_the_last_entry_of_each_key_survives() {
    // Two windows resized in one batch, with unrelated events between their sizes.
    let keys = [
        Some('a'),
        None,
        Some('b'),
        Some('a'),
        None,
        Some('b'),
        Some('b'),
    ];
    assert_eq!(
        superseded(&keys),
        [true, false, true, false, false, true, false]
    );
}

#[test]
fn a_batch_with_one_entry_per_key_loses_nothing() {
    assert_eq!(superseded(&[Some(1), None, Some(2)]), [false, false, false]);
    assert_eq!(superseded::<u32>(&[]), Vec::<bool>::new());
}
