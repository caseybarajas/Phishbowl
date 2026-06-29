use std::path::PathBuf;

use cli::pipeline::{referee_step, StepOutcome};
use world::{
    Ask, ParsedAction, PersonaId, Principle, Rule, SecretKind, SuccessCondition, Verdict, World,
};

fn priya_world() -> World {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/it-support-credential/scenario.ron");
    scenario::load(&path).expect("starter scenario loads")
}

fn identity(authority: &str, principle: Principle) -> ParsedAction {
    ParsedAction {
        authority_claim: Some(authority.to_owned()),
        principles: vec![principle],
        ..ParsedAction::inert()
    }
}

fn rapport(principle: Principle) -> ParsedAction {
    ParsedAction {
        principles: vec![principle],
        ..ParsedAction::inert()
    }
}

fn cite(reference: &str) -> ParsedAction {
    ParsedAction {
        verification: Some(reference.to_owned()),
        ..ParsedAction::inert()
    }
}

fn ask_door_code() -> ParsedAction {
    ParsedAction {
        ask: Some(Ask {
            kind: SecretKind::DoorCode,
            target: None,
            sensitivity_hint: 0,
        }),
        ..ParsedAction::inert()
    }
}

fn suspicion(world: &World, contact: &PersonaId) -> u8 {
    world.persona(contact).unwrap().state.suspicion.get()
}

fn trust(world: &World, contact: &PersonaId) -> u8 {
    world.persona(contact).unwrap().state.trust.get()
}

/// The end-to-end balance proof the tuning fixes are for: posing as IT, a patient
/// operator who builds rapport and *cites a checkable ticket* lifts Priya's trust past
/// the disclosure threshold while her suspicion stays well under the detection ceiling —
/// and the door code is granted. Pure referee, no model.
#[test]
fn patient_verifiable_run_reaches_disclosure_before_detection() {
    let mut world = priya_world();
    let contact = PersonaId::from("priya_v");

    let detection = world.tuning.detection_threshold.get();
    let report = world.tuning.report_threshold.get();
    let ceiling = detection.min(report);

    let SuccessCondition::SecretDisclosed(goal) = world.objective.success.clone();
    let rule = world
        .secret(&goal)
        .expect("objective secret exists")
        .disclosure
        .clone();
    let (trust_min, suspicion_max) = (rule.trust_min.get(), rule.suspicion_max.get());

    let mut turn = 0u32;
    let mut step = |world: &mut World, mut action: ParsedAction| -> StepOutcome {
        turn += 1;
        let out = referee_step(world, &contact, &mut action, turn);
        // Suspicion must never approach the detection/report ceiling on a clean run.
        assert!(
            suspicion(world, &contact) < ceiling,
            "turn {turn}: suspicion {} hit the ceiling {ceiling}",
            suspicion(world, &contact)
        );
        out
    };

    // 1. Establish the IT-helpdesk identity. Scrutinized once.
    step(
        &mut world,
        identity("Alex, Northwind IT helpdesk", Principle::Liking),
    );
    let after_establish = suspicion(&world, &contact);
    assert!(
        after_establish > 8,
        "establishing the claim should cost something once"
    );

    // 2. Re-assert the same identity: a standing condition must not compound.
    let out = step(
        &mut world,
        identity("Alex, Northwind IT helpdesk", Principle::Reciprocity),
    );
    assert_eq!(
        out.appraisal.suspicion_delta, 0,
        "a re-referenced standing claim must not add suspicion"
    );
    assert_eq!(suspicion(&world, &contact), after_establish);

    // 3-4. Patient rapport, no asks — trust climbs to the disclosure threshold.
    step(&mut world, rapport(Principle::SocialProof));
    step(&mut world, rapport(Principle::Reciprocity));
    assert!(
        trust(&world, &contact) >= trust_min,
        "rapport should reach the trust bar"
    );

    // 5. Cite the checkable ticket: standing authority suspicion is relieved.
    let before_cite = suspicion(&world, &contact);
    let out = step(&mut world, cite("INC-4471"));
    assert!(fired(&out, Rule::AuthorityVerified));
    assert!(
        suspicion(&world, &contact) < before_cite,
        "a checkable reference must lower standing suspicion"
    );

    // 6. Now ask for the door code — every gate is satisfied.
    let out = step(&mut world, ask_door_code());
    assert_eq!(
        out.verdict,
        Some(Verdict::Grant),
        "the patient run should be granted"
    );
    assert!(trust(&world, &contact) >= trust_min);
    assert!(suspicion(&world, &contact) < suspicion_max);
    assert!(suspicion(&world, &contact) < detection);
}

fn fired(out: &StepOutcome, rule: Rule) -> bool {
    out.appraisal.reasons.iter().any(|r| r.rule == rule)
        || out.verdict_reasons.iter().any(|r| r.rule == rule)
}
