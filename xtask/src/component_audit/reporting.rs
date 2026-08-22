//! Baseline comparison and human-readable component-audit reporting.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    ComponentAuditReport, ComponentClass, ComponentEntry, ContractCheck, ContractStatus,
    MetricPolicy, SourceMetric,
};

/// Compares a current audit report with its baseline and returns every regression.
pub(super) fn compare_with_baseline(
    baseline: &ComponentAuditReport,
    current: &ComponentAuditReport,
    approved_migrations: &BTreeSet<(String, ComponentClass, ComponentClass)>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if baseline.version != current.version {
        failures.push(format!(
            "baseline version {} != current version {}",
            baseline.version, current.version
        ));
    }

    let baseline_components = keyed_components(&baseline.components);
    let current_components = keyed_components(&current.components);
    for concept in baseline_components.keys() {
        if !current_components.contains_key(concept) {
            failures.push(format!("component manifest entry removed: {concept}"));
        }
    }
    for (concept, current_entry) in &current_components {
        if let Some(baseline_entry) = baseline_components.get(concept) {
            if baseline_entry.class != current_entry.class {
                let approved = approved_migrations.contains(&(
                    concept.to_string(),
                    baseline_entry.class,
                    current_entry.class,
                ));
                if !approved {
                    failures.push(format!(
                        "component {concept} class changed {:?} -> {:?}; record an approved migration first",
                        baseline_entry.class, current_entry.class
                    ));
                }
            }
        }
    }

    let baseline_metrics = keyed_metrics(&baseline.source_metrics);
    let current_metrics = keyed_metrics(&current.source_metrics);
    for (id, current_metric) in current_metrics {
        if let Some(baseline_metric) = baseline_metrics.get(id) {
            match current_metric.policy {
                MetricPolicy::MustNotIncrease | MetricPolicy::MustBeZeroEventually => {
                    if current_metric.count > baseline_metric.count {
                        failures.push(format!(
                            "metric {id} regressed: {} -> {}",
                            baseline_metric.count, current_metric.count
                        ));
                    }
                }
                MetricPolicy::Informational => {}
            }
        }
    }

    let baseline_contracts = keyed_contracts(&baseline.contracts);
    let current_contracts = keyed_contracts(&current.contracts);
    for (id, baseline_contract) in baseline_contracts {
        let Some(current_contract) = current_contracts.get(id) else {
            failures.push(format!("contract removed: {id}"));
            continue;
        };
        if baseline_contract.status == ContractStatus::Pass
            && current_contract.status != ContractStatus::Pass
        {
            failures.push(format!(
                "contract {id} regressed: Pass -> {:?}",
                current_contract.status
            ));
        }
    }

    failures
}

/// Index component entries by their stable manifest concept.
fn keyed_components(entries: &[ComponentEntry]) -> BTreeMap<&str, &ComponentEntry> {
    entries
        .iter()
        .map(|entry| (entry.concept.as_str(), entry))
        .collect()
}

/// Index source metrics by their stable audit identifier.
fn keyed_metrics(entries: &[SourceMetric]) -> BTreeMap<&str, &SourceMetric> {
    entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect()
}

/// Index contract checks by their stable contract identifier.
fn keyed_contracts(entries: &[ContractCheck]) -> BTreeMap<&str, &ContractCheck> {
    entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect()
}

/// Prints the human-readable report and any accumulated failures.
pub(super) fn print_human_report(report: &ComponentAuditReport, failures: &[String]) {
    let mut classes = BTreeMap::<ComponentClass, usize>::new();
    for entry in &report.components {
        *classes.entry(entry.class).or_default() += 1;
    }
    println!("MoonUI component audit v{}", report.version);
    println!("components:");
    for (class, count) in classes {
        println!("  {class:?}: {count}");
    }
    println!("source metrics:");
    for metric in &report.source_metrics {
        println!("  {} = {} ({:?})", metric.id, metric.count, metric.policy);
    }
    println!("contracts:");
    for contract in &report.contracts {
        println!(
            "  {:?} {:?} {:?} {} - {}",
            contract.status, contract.severity, contract.verifier, contract.id, contract.details
        );
    }
    if failures.is_empty() {
        println!("component audit: PASS");
    } else {
        println!("component audit: FAIL");
        for failure in failures {
            println!("  - {failure}");
        }
    }
}
