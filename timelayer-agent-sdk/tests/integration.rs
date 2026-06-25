use std::path::Path;
use timelayer_agent_sdk::{AgentBundle, CheckResult, StopReason};

const EXAMPLE_BUNDLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../example-bundle"
);

const VERIFIER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../TimeLayer-Demo/receipt-driven/bin/timelayer-verifier"
);

fn load_bundle() -> AgentBundle {
    AgentBundle::load_with_verifier(
        Path::new(EXAMPLE_BUNDLE),
        Some(Path::new(VERIFIER)),
    )
    .expect("failed to load example-bundle")
}

#[test]
fn bundle_loads_successfully() {
    let bundle = load_bundle();
    assert_eq!(bundle.manifest.receipt_count, 4);
    assert_eq!(bundle.manifest.topology_id, "topo_project_review_001");
    assert!(!bundle.manifest.agent_can_issue_receipts);
    assert!(bundle.manifest.no_receipt_no_action);
}

#[test]
fn all_actions_allow_with_valid_receipts() {
    let bundle = load_bundle();
    for action_id in bundle.action_ids() {
        match bundle.check_action(action_id) {
            CheckResult::Allow { .. } => {}
            CheckResult::Stop { reason, message, .. } => {
                panic!("action '{action_id}' should be ALLOW but got STOP({reason}): {message}");
            }
        }
    }
}

#[test]
fn missing_action_returns_stop_no_receipt() {
    let bundle = load_bundle();
    match bundle.check_action("action_does_not_exist") {
        CheckResult::Stop { reason, .. } => {
            assert_eq!(reason, StopReason::NoReceipt);
        }
        CheckResult::Allow { .. } => {
            panic!("nonexistent action should return Stop(NoReceipt)");
        }
    }
}

#[test]
fn topology_next_actions_are_correct() {
    let bundle = load_bundle();

    let next = bundle.allowed_next("action_read_files");
    assert!(next.contains(&"action_summarize".to_string()));
    assert!(next.contains(&"action_stop_for_review".to_string()));

    let next2 = bundle.allowed_next("action_create_report");
    assert_eq!(next2, vec!["action_stop_for_review"]);

    // stop has no next
    let next3 = bundle.allowed_next("action_stop_for_review");
    assert!(next3.is_empty());
}

#[test]
fn transition_check_valid_path() {
    let bundle = load_bundle();
    match bundle.check_transition("action_read_files", "action_summarize") {
        CheckResult::Allow { .. } => {}
        r => panic!("expected Allow, got {r:?}"),
    }
}

#[test]
fn transition_check_invalid_path() {
    let bundle = load_bundle();
    // create_report cannot go back to read_files
    match bundle.check_transition("action_create_report", "action_read_files") {
        CheckResult::Stop { reason, .. } => {
            assert_eq!(reason, StopReason::TopologyDenied);
        }
        r => panic!("expected Stop(TopologyDenied), got {r:?}"),
    }
}

#[test]
fn audit_all_passes() {
    let bundle = load_bundle();
    let report = bundle.audit();
    assert_eq!(report.len(), 4);
    for entry in &report {
        assert!(
            entry.is_valid(),
            "action '{}' failed audit: {:?}",
            entry.action_id,
            entry.result
        );
    }
}

#[test]
fn segment_bundle_loads() {
    let segment_path = Path::new(EXAMPLE_BUNDLE).join("exports/segment_01");
    let bundle = AgentBundle::load_with_verifier(
        &segment_path,
        Some(Path::new(VERIFIER)),
    )
    .expect("failed to load segment_01");

    assert_eq!(bundle.manifest.bundle_type, "segment");
    assert_eq!(bundle.manifest.actions.len(), 1);

    match bundle.check_action("action_read_files") {
        CheckResult::Allow { .. } => {}
        r => panic!("segment action should be ALLOW, got {r:?}"),
    }
}

#[test]
fn verifier_available() {
    let bundle = load_bundle();
    assert!(
        bundle.verifier_available(),
        "verifier or roster not found"
    );
}
