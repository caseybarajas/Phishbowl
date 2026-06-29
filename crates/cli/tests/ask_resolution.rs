use std::path::PathBuf;

use cli::pipeline::referee_step;
use world::{Ask, ParsedAction, PersonaId, Rule, SecretKind, Suspicion, Trust, Verdict};

fn priya_world() -> world::World {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/it-support-credential/scenario.ron");
    scenario::load(&path).expect("starter scenario loads")
}

/// The live failure: Analyst classifies "VPN enrollment code" as `Info` (or anything
/// other than `DoorCode`), so kind-only resolution misses `priya_vpn_code` entirely. Referent
/// matching must land the objective secret and grant when trust/suspicion qualify.
#[test]
fn vpn_enrollment_code_resolves_to_objective_secret_and_grants() {
    let mut world = priya_world();
    let contact = PersonaId::from("priya_v");

    let persona = world.persona_mut(&contact).expect("priya exists");
    persona.state.trust = Trust::new(50);
    persona.state.suspicion = Suspicion::new(20);

    let mut action = ParsedAction {
        ask: Some(Ask {
            kind: SecretKind::Info,
            referent: Some("VPN enrollment code".into()),
            target: None,
            sensitivity_hint: 80,
        }),
        ..ParsedAction::inert()
    };

    let out = referee_step(&mut world, &contact, &mut action, 1);
    let ask = action.ask.expect("ask present");

    assert_eq!(
        ask.target.as_ref().map(world::SecretId::as_str),
        Some("priya_vpn_code"),
        "referent must resolve to the VPN enrollment secret, not password"
    );
    assert_ne!(
        ask.target.as_ref().map(world::SecretId::as_str),
        Some("priya_password")
    );
    assert_eq!(out.verdict, Some(Verdict::Grant));
}

#[test]
fn unresolved_ask_deflects_without_escalation_penalty() {
    let mut world = priya_world();
    let contact = PersonaId::from("priya_v");

    let mut action = ParsedAction {
        ask: Some(Ask {
            kind: SecretKind::Info,
            referent: Some("the thing from earlier".into()),
            target: None,
            sensitivity_hint: 95,
        }),
        ..ParsedAction::inert()
    };

    let out = referee_step(&mut world, &contact, &mut action, 1);

    assert!(action.ask.as_ref().unwrap().target.is_none());
    assert_eq!(out.verdict, Some(Verdict::Deflect));
    assert!(!out
        .appraisal
        .reasons
        .iter()
        .any(|r| r.rule == Rule::EscalationSpeed));
}
