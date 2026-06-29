use std::path::PathBuf;

use world::{RunStatus, SuccessCondition};

use crate::{instantiate, load, LintError, ScenarioError, ScenarioPackage};

const BASE: &str = r#"(
    meta: (id: "t", title: "T", author: "a", premise: "p", difficulty: "normal"),
    organization: (
        name: "Org",
        industry: "Logistics",
        formality: Neutral,
        security_conscious: true,
        policies: [(id: "no-pw", description: "IT never asks for passwords", forbids_disclosure_of: Password)],
    ),
    cast: [
        (
            id: "priya",
            name: "Priya",
            title: "Accountant",
            department: "Finance",
            personality: (agreeableness: 70, conscientiousness: 45, security_awareness: 35, busyness: 60, ego: 40, helpfulness: 75),
            voice: (style: "warm, brief", formality: Casual, avg_words: 15),
            knowledge: ["vpn"],
            start: (suspicion: 10, trust: 10, mood: Neutral),
        ),
    ],
    secrets: [
        (
            id: "vpn",
            owner: "priya",
            kind: DoorCode,
            label: "VPN enrollment code",
            aliases: ["enrollment code", "vpn code"],
            sensitivity: 70,
            value: "VPN-7731",
            disclosure: (trust_min: 45, suspicion_max: 50, required_pretext: Some("IT Support"), requires_authorization: false),
        ),
    ],
    objective: (description: "get the vpn code", success: SecretDisclosed("vpn"), turn_budget: 20),
    seed: (
        pretext: Some((label: "IT Support", claimed_identity: "Alex from IT", internal_claim: true, verifiable: false)),
        intel: [],
    ),
)"#;

fn parse(text: &str) -> Result<world::World, ScenarioError> {
    let pkg = ron::from_str::<ScenarioPackage>(text)?;
    Ok(instantiate(pkg)?)
}

fn lint_err(text: &str) -> LintError {
    match parse(text) {
        Err(ScenarioError::Lint(e)) => e,
        other => panic!("expected a lint error, got {other:?}"),
    }
}

#[test]
fn valid_package_instantiates() {
    let world = parse(BASE).expect("base package should be valid");
    assert_eq!(world.status, RunStatus::Active);
    assert_eq!(world.personas.len(), 1);
    assert!(matches!(
        world.objective.success,
        SuccessCondition::SecretDisclosed(_)
    ));
    assert_eq!(world.player.pretext.unwrap().label, "IT Support");
}

#[test]
fn dangling_secret_owner_is_rejected() {
    let text = BASE.replace(r#"owner: "priya""#, r#"owner: "ghost""#);
    assert!(matches!(
        lint_err(&text),
        LintError::DanglingSecretOwner { .. }
    ));
}

#[test]
fn unknown_objective_secret_is_rejected() {
    let text = BASE.replace(r#"SecretDisclosed("vpn")"#, r#"SecretDisclosed("nope")"#);
    assert_eq!(
        lint_err(&text),
        LintError::UnknownObjectiveSecret("nope".into())
    );
}

#[test]
fn objective_blocked_by_policy_is_unreachable() {
    let text = BASE.replace("kind: DoorCode", "kind: Password");
    assert!(matches!(lint_err(&text), LintError::Unreachable { .. }));
}

#[test]
fn objective_needing_absent_pretext_is_unreachable() {
    let text = BASE.replace(
        r#"required_pretext: Some("IT Support")"#,
        r#"required_pretext: Some("Vendor")"#,
    );
    assert!(matches!(lint_err(&text), LintError::Unreachable { .. }));
}

#[test]
fn owner_above_suspicion_ceiling_is_unreachable() {
    let text = BASE.replace("suspicion: 10", "suspicion: 60");
    assert!(matches!(lint_err(&text), LintError::Unreachable { .. }));
}

#[test]
fn dangling_knowledge_ref_is_rejected() {
    let text = BASE.replace(r#"knowledge: ["vpn"]"#, r#"knowledge: ["vpn", "missing"]"#);
    assert!(matches!(
        lint_err(&text),
        LintError::DanglingKnowledge { .. }
    ));
}

#[test]
fn shipped_starter_scenario_lints_clean() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/it-support-credential/scenario.ron");
    let world = load(&path).expect("starter scenario should load and lint clean");
    assert!(matches!(
        world.objective.success,
        SuccessCondition::SecretDisclosed(_)
    ));
}
