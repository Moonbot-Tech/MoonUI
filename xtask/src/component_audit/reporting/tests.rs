//! Regression tests for component-audit baseline comparison.

use std::collections::BTreeSet;

use super::compare_with_baseline;
use crate::component_audit::{
    ComponentAuditReport, ComponentClass, ComponentEntry, ContractCheck, ContractSeverity,
    ContractStatus, ContractVerifier, MetricPolicy, SourceMetric,
};

fn component(concept: &str, class: ComponentClass) -> ComponentEntry {
    ComponentEntry {
        concept: concept.to_string(),
        class,
        behavior_source: "behavior".to_string(),
        theme_source: "theme".to_string(),
        public_path: None,
        escape_path: None,
        upstream_ref: None,
        fork_reason: None,
        donor_drift_budget: None,
        contracts: Vec::new(),
    }
}

fn metric(id: &str, count: usize, policy: MetricPolicy) -> SourceMetric {
    SourceMetric {
        id: id.to_string(),
        count,
        policy,
        hits: Vec::new(),
    }
}

fn contract(id: &str, status: ContractStatus) -> ContractCheck {
    ContractCheck {
        id: id.to_string(),
        status,
        severity: ContractSeverity::Guardrail,
        verifier: ContractVerifier::BehavioralTest,
        details: "details".to_string(),
    }
}

fn report(
    components: Vec<ComponentEntry>,
    source_metrics: Vec<SourceMetric>,
    contracts: Vec<ContractCheck>,
) -> ComponentAuditReport {
    ComponentAuditReport {
        version: 1,
        components,
        source_metrics,
        contracts,
    }
}

/// Catches deleting the version check in `compare_with_baseline`, which would
/// grade a report against a baseline written for a different audit schema.
#[test]
fn version_mismatch_is_a_regression() {
    let mut current = report(Vec::new(), Vec::new(), Vec::new());
    current.version = 2;

    let failures = compare_with_baseline(
        &report(Vec::new(), Vec::new(), Vec::new()),
        &current,
        &BTreeSet::new(),
    );

    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(
        failures[0].contains("baseline version 1") && failures[0].contains("current version 2"),
        "{failures:?}"
    );
}

/// Catches skipping a concept that disappeared from the manifest, which would
/// let a component vanish from the audit. A concept only the current report
/// has is not a regression.
#[test]
fn removed_component_fails_and_an_addition_does_not() {
    let button = component("button", ComponentClass::Forged);
    let alert = component("alert", ComponentClass::Forged);
    let slider = component("slider", ComponentClass::Forged);

    let removed = compare_with_baseline(
        &report(vec![button.clone(), alert], Vec::new(), Vec::new()),
        &report(vec![button.clone()], Vec::new(), Vec::new()),
        &BTreeSet::new(),
    );
    assert_eq!(removed.len(), 1, "{removed:?}");
    assert!(
        removed[0].contains("removed") && removed[0].contains("alert"),
        "{removed:?}"
    );

    let added = compare_with_baseline(
        &report(vec![button.clone()], Vec::new(), Vec::new()),
        &report(vec![button, slider], Vec::new(), Vec::new()),
        &BTreeSet::new(),
    );
    assert!(added.is_empty(), "{added:?}");
}

/// Catches ignoring the approved-migration set, which would either block a
/// recorded Mirror to TrackedFork move or let that move land with no record.
/// A record of the opposite direction does not approve it.
#[test]
fn class_change_requires_a_recorded_migration() {
    let baseline = report(
        vec![component("button", ComponentClass::Mirror)],
        Vec::new(),
        Vec::new(),
    );
    let current = report(
        vec![component("button", ComponentClass::TrackedFork)],
        Vec::new(),
        Vec::new(),
    );

    let unapproved = compare_with_baseline(&baseline, &current, &BTreeSet::new());
    assert_eq!(unapproved.len(), 1, "{unapproved:?}");
    assert!(
        unapproved[0].contains("button") && unapproved[0].contains("class changed"),
        "{unapproved:?}"
    );

    let mut approved = BTreeSet::new();
    approved.insert((
        "button".to_string(),
        ComponentClass::Mirror,
        ComponentClass::TrackedFork,
    ));
    let recorded = compare_with_baseline(&baseline, &current, &approved);
    assert!(recorded.is_empty(), "{recorded:?}");

    let mut reversed = BTreeSet::new();
    reversed.insert((
        "button".to_string(),
        ComponentClass::TrackedFork,
        ComponentClass::Mirror,
    ));
    let wrong_direction = compare_with_baseline(&baseline, &current, &reversed);
    assert_eq!(wrong_direction.len(), 1, "{wrong_direction:?}");
    assert!(
        wrong_direction[0].contains("class changed"),
        "{wrong_direction:?}"
    );
}

/// Catches flipping the guarded-metric comparison, which would accept a count
/// that grew and fail a count that shrank. `MustNotIncrease` and
/// `MustBeZeroEventually` share that comparison.
#[test]
fn guarded_metric_rejects_a_higher_count() {
    let grew = compare_with_baseline(
        &report(
            Vec::new(),
            vec![metric("raw_hex", 1, MetricPolicy::MustNotIncrease)],
            Vec::new(),
        ),
        &report(
            Vec::new(),
            vec![metric("raw_hex", 2, MetricPolicy::MustNotIncrease)],
            Vec::new(),
        ),
        &BTreeSet::new(),
    );
    assert_eq!(grew.len(), 1, "{grew:?}");
    assert!(
        grew[0].contains("raw_hex") && grew[0].contains("regressed") && grew[0].contains("1 -> 2"),
        "{grew:?}"
    );

    let shrank = compare_with_baseline(
        &report(
            Vec::new(),
            vec![metric("raw_hex", 2, MetricPolicy::MustNotIncrease)],
            Vec::new(),
        ),
        &report(
            Vec::new(),
            vec![metric("raw_hex", 1, MetricPolicy::MustNotIncrease)],
            Vec::new(),
        ),
        &BTreeSet::new(),
    );
    assert!(shrank.is_empty(), "{shrank:?}");

    let unchanged = compare_with_baseline(
        &report(
            Vec::new(),
            vec![metric("raw_hex", 1, MetricPolicy::MustNotIncrease)],
            Vec::new(),
        ),
        &report(
            Vec::new(),
            vec![metric("raw_hex", 1, MetricPolicy::MustNotIncrease)],
            Vec::new(),
        ),
        &BTreeSet::new(),
    );
    assert!(unchanged.is_empty(), "{unchanged:?}");

    let zero_grew = compare_with_baseline(
        &report(
            Vec::new(),
            vec![metric("todo", 0, MetricPolicy::MustBeZeroEventually)],
            Vec::new(),
        ),
        &report(
            Vec::new(),
            vec![metric("todo", 1, MetricPolicy::MustBeZeroEventually)],
            Vec::new(),
        ),
        &BTreeSet::new(),
    );
    assert_eq!(zero_grew.len(), 1, "{zero_grew:?}");
    assert!(
        zero_grew[0].contains("todo") && zero_grew[0].contains("0 -> 1"),
        "{zero_grew:?}"
    );
}

/// Catches treating an informational metric like a guarded one, which would
/// fail the audit when a count that is only reported grows.
#[test]
fn informational_metric_may_grow() {
    let failures = compare_with_baseline(
        &report(
            Vec::new(),
            vec![metric("notes", 1, MetricPolicy::Informational)],
            Vec::new(),
        ),
        &report(
            Vec::new(),
            vec![metric("notes", 5, MetricPolicy::Informational)],
            Vec::new(),
        ),
        &BTreeSet::new(),
    );

    assert!(failures.is_empty(), "{failures:?}");
}

/// Catches inverting the contract status check, which would flag a contract
/// that stayed passing and miss one that left Pass. A contract that moves
/// from Fail to Pass is not a regression, and leaving Pass for Debt is.
#[test]
fn passing_contract_that_stops_passing_is_a_regression() {
    let regressed = compare_with_baseline(
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Pass)],
        ),
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Fail)],
        ),
        &BTreeSet::new(),
    );
    assert_eq!(regressed.len(), 1, "{regressed:?}");
    assert!(
        regressed[0].contains("focus")
            && regressed[0].contains("Pass")
            && regressed[0].contains("Fail"),
        "{regressed:?}"
    );

    let still_passing = compare_with_baseline(
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Pass)],
        ),
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Pass)],
        ),
        &BTreeSet::new(),
    );
    assert!(still_passing.is_empty(), "{still_passing:?}");

    let improved = compare_with_baseline(
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Fail)],
        ),
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Pass)],
        ),
        &BTreeSet::new(),
    );
    assert!(improved.is_empty(), "{improved:?}");

    let debt = compare_with_baseline(
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Pass)],
        ),
        &report(
            Vec::new(),
            Vec::new(),
            vec![contract("focus", ContractStatus::Debt)],
        ),
        &BTreeSet::new(),
    );
    assert_eq!(debt.len(), 1, "{debt:?}");
    assert!(debt[0].contains("Debt"), "{debt:?}");
}
