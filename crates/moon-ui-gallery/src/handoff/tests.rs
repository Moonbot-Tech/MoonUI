use std::collections::HashSet;

use super::{HANDOFF_CASES, selected_handoff_case_indices};

/// Catches duplicate or path-bearing IDs overwriting or escaping their intended snapshot PNG.
#[test]
fn handoff_case_ids_are_unique_and_filename_safe() {
    let mut ids = HashSet::new();
    for case in HANDOFF_CASES {
        assert!(
            ids.insert(case.id),
            "duplicate handoff case ID: {}",
            case.id
        );
        assert!(
            case.id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')),
            "handoff case ID is not filename-safe: {}",
            case.id
        );
    }
}

/// Catches selection changes that reorder explicitly requested snapshot cases.
#[test]
fn handoff_case_selection_preserves_requested_order() {
    let requested = vec![
        "tag.variants".to_string(),
        "button.neutral".to_string(),
        "status_bar.basic".to_string(),
    ];

    let selected = selected_handoff_case_indices(&requested)
        .into_iter()
        .map(|ix| HANDOFF_CASES[ix].id)
        .collect::<Vec<_>>();

    assert_eq!(
        selected,
        ["tag.variants", "button.neutral", "status_bar.basic"]
    );
}

/// Catches an all-unknown filter producing no captures instead of the documented full-set fallback.
#[test]
fn handoff_case_selection_falls_back_when_all_ids_are_unknown() {
    let requested = vec!["missing.case".to_string(), "also.missing".to_string()];

    let selected = selected_handoff_case_indices(&requested);
    let full_set = (0..HANDOFF_CASES.len()).collect::<Vec<_>>();

    assert_eq!(selected, full_set);
}
