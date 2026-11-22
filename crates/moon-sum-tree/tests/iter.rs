//! Integration tests for the public `SumTree::iter` iterator.

use moon_sum_tree::{ContextLessSummary, Item, SumTree};

#[derive(Clone, Debug)]
struct Unit;

#[derive(Clone, Debug, Default)]
struct Count(usize);

impl ContextLessSummary for Count {
    fn zero() -> Self {
        Self::default()
    }

    fn add_summary(&mut self, other: &Self) {
        self.0 += other.0;
    }
}

impl Item for Unit {
    type Summary = Count;

    fn summary(&self, _: ()) -> Count {
        Count(1)
    }
}

/// Catches `Iter::size_hint` counting the leaf item `next` just returned as
/// still to come, so after every `next` the lower bound is one more than the
/// items left — a one-item tree reports `(1, None)` once it has nothing left.
///
/// The expected bound comes from the `Iterator::size_hint` contract in `std`:
/// the iterator yields at least `lower` more elements, so `lower` may never
/// exceed the count of items not yet yielded, which the loop counts by hand.
#[test]
fn size_hint_lower_bound_never_exceeds_the_items_left() {
    for len in 0..60 {
        let tree = SumTree::from_iter(std::iter::repeat_n(Unit, len), ());
        let mut iter = tree.iter();
        for yielded in 0..=len {
            let left = len - yielded;
            let (lower, _) = iter.size_hint();
            assert!(
                lower <= left,
                "tree of {len}: lower bound {lower} after {yielded} items, only {left} left"
            );
            if left > 0 {
                assert!(iter.next().is_some());
            }
        }
    }
}
