use world::{
    Adjudication, Appraisal, Ask, Beliefs, Channel, ChannelKind, Claim, Coherence, Culture,
    DisclosureRule, Formality, Mood, Objective, Organization, ParsedAction, Persona, PersonaState,
    Personality, Policy, Pretext, Principle, Rule, RunStatus, SalientFact, Secret, SecretKind,
    SuccessCondition, Suspicion, Trust, Verdict, Voice, World,
};

use crate::{adjudicate, appraise};

fn ax(value: u8) -> world::Axis {
    world::Axis::new(value)
}

fn neutral_personality() -> Personality {
    Personality {
        agreeableness: ax(50),
        conscientiousness: ax(50),
        security_awareness: ax(50),
        busyness: ax(50),
        ego: ax(50),
        helpfulness: ax(50),
    }
}

fn persona(personality: Personality) -> Persona {
    Persona {
        id: "target".into(),
        name: "Target".into(),
        title: "Analyst".into(),
        department: "Finance".into(),
        reports_to: None,
        personality,
        voice: Voice {
            style: "plain".into(),
            formality: Formality::Neutral,
            avg_words: 20,
            quirks: vec![],
            emoji: false,
        },
        knowledge: vec!["vpn".into()],
        facts: vec![],
        relationships: vec![],
        hooks: vec![],
        red_lines: vec![],
        state: PersonaState {
            suspicion: Suspicion::new(10),
            trust: Trust::new(10),
            mood: Mood::Neutral,
            beliefs: Beliefs::default(),
            principle_history: vec![],
        },
    }
}

fn vpn_secret() -> Secret {
    Secret {
        id: "vpn".into(),
        owner: "target".into(),
        kind: SecretKind::DoorCode,
        sensitivity: world::Sensitivity::new(70),
        value: "VPN-7731".into(),
        disclosure: DisclosureRule {
            trust_min: Trust::new(45),
            suspicion_max: Suspicion::new(50),
            required_pretext: Some("IT Support".into()),
            requires_authorization: false,
        },
    }
}

fn password_policy() -> Policy {
    Policy {
        id: "no-pw".into(),
        description: "IT will never ask for your password".into(),
        forbids_disclosure_of: SecretKind::Password,
    }
}

fn world(persona: Persona, policies: Vec<Policy>) -> World {
    World {
        org: Organization {
            name: "Northwind".into(),
            industry: "Logistics".into(),
            culture: Culture {
                formality: Formality::Neutral,
                security_conscious: true,
            },
            policies,
        },
        personas: vec![persona],
        secrets: vec![vpn_secret()],
        facts: vec![],
        channels: vec![Channel {
            id: "dm".into(),
            kind: ChannelKind::Messenger,
            messages: vec![],
        }],
        player: world::Player::default(),
        objective: Objective {
            description: "obtain VPN code".into(),
            success: SuccessCondition::SecretDisclosed("vpn".into()),
            turn_budget: 20,
        },
        tuning: world::Tuning::default(),
        clock: 0,
        org_suspicion: Suspicion::new(0),
        status: RunStatus::Active,
        log: world::CausalLog::default(),
    }
}

fn it_pretext() -> Pretext {
    Pretext {
        label: "IT Support".into(),
        claimed_identity: "Alex from IT".into(),
        internal_claim: true,
        verifiable: false,
    }
}

fn action() -> ParsedAction {
    ParsedAction::inert()
}

fn fired(reasons: &[world::CausalEntry], rule: Rule) -> bool {
    reasons.iter().any(|r| r.rule == rule)
}

fn appraise_with(world: &World, action: &ParsedAction) -> Appraisal {
    let p = &world.personas[0];
    appraise(world, p, action)
}

fn adjudicate_with(world: &World, ask: &Ask) -> Adjudication {
    let p = &world.personas[0];
    adjudicate(world, p, ask)
}

#[test]
fn inert_parse_is_a_no_op() {
    let w = world(persona(neutral_personality()), vec![]);
    let out = appraise_with(&w, &action());
    assert_eq!(out.suspicion_delta, 0);
    assert_eq!(out.trust_delta, 0);
    assert!(out.reasons.is_empty());
}

#[test]
fn fourth_wall_spikes_suspicion() {
    let w = world(persona(neutral_personality()), vec![]);
    let mut a = action();
    a.coherence = Coherence::Anomalous;
    let out = appraise_with(&w, &a);
    assert_eq!(out.suspicion_delta, w.tuning.w_fourth_wall);
    assert!(fired(&out.reasons, Rule::FourthWall));
}

#[test]
fn contradiction_of_stored_fact_raises_suspicion() {
    let mut p = persona(neutral_personality());
    p.state.beliefs.salient_facts.push(SalientFact {
        key: "office".into(),
        value: "Houston".into(),
        turn: 1,
    });
    let w = world(p, vec![]);
    let mut a = action();
    a.claims.push(Claim {
        key: "office".into(),
        value: "Dallas".into(),
    });
    let out = appraise_with(&w, &a);
    assert!(out.suspicion_delta > 0);
    assert!(fired(&out.reasons, Rule::Inconsistency));
}

#[test]
fn matching_claim_does_not_trip_inconsistency() {
    let mut p = persona(neutral_personality());
    p.state.beliefs.salient_facts.push(SalientFact {
        key: "office".into(),
        value: "Houston".into(),
        turn: 1,
    });
    let w = world(p, vec![]);
    let mut a = action();
    a.claims.push(Claim {
        key: "office".into(),
        value: "Houston".into(),
    });
    let out = appraise_with(&w, &a);
    assert!(!fired(&out.reasons, Rule::Inconsistency));
}

#[test]
fn password_ask_violates_policy() {
    let w = world(persona(neutral_personality()), vec![password_policy()]);
    let mut a = action();
    a.ask = Some(Ask {
        kind: SecretKind::Password,
        target: None,
        sensitivity_hint: 80,
    });
    let out = appraise_with(&w, &a);
    assert!(fired(&out.reasons, Rule::PolicyViolation));
}

#[test]
fn internal_claim_from_outside_is_an_oddity_and_unverifiable_authority() {
    let mut w = world(persona(neutral_personality()), vec![]);
    w.player.pretext = Some(it_pretext());
    let mut a = action();
    a.authority_claim = Some("IT helpdesk".into());
    let out = appraise_with(&w, &a);
    assert!(fired(&out.reasons, Rule::ChannelOddity));
    assert!(fired(&out.reasons, Rule::AuthorityMismatch));
}

#[test]
fn escalation_speed_scales_with_gap() {
    let w = world(persona(neutral_personality()), vec![]);
    let mut a = action();
    a.ask = Some(Ask {
        kind: SecretKind::DoorCode,
        target: Some("vpn".into()),
        sensitivity_hint: 0,
    });
    let out = appraise_with(&w, &a);
    assert!(fired(&out.reasons, Rule::EscalationSpeed));
    // sensitivity 70 - trust 10 = 60, * 30% = 18
    let escalation = out
        .reasons
        .iter()
        .find(|r| r.rule == Rule::EscalationSpeed)
        .unwrap()
        .weight;
    assert_eq!(escalation, 18);
}

#[test]
fn repeated_principle_flips_to_reactance() {
    let mut p = persona(neutral_personality());
    p.state.principle_history = vec![Principle::Scarcity, Principle::Scarcity];
    let w = world(p, vec![]);
    let mut a = action();
    a.principles.push(Principle::Scarcity);
    let out = appraise_with(&w, &a);
    assert!(fired(&out.reasons, Rule::OverPressure));
    assert!(!fired(&out.reasons, Rule::PrincipleFit));
}

#[test]
fn flattery_lands_harder_on_high_ego() {
    let mut high = neutral_personality();
    high.ego = ax(100);
    let w_high = world(persona(high), vec![]);
    let w_low = world(
        persona({
            let mut low = neutral_personality();
            low.ego = ax(0);
            low
        }),
        vec![],
    );
    let mut a = action();
    a.principles.push(Principle::Liking);

    let high_gain = appraise_with(&w_high, &a).trust_delta;
    let low_gain = appraise_with(&w_low, &a).trust_delta;
    assert!(high_gain > low_gain);
}

#[test]
fn substantive_small_talk_builds_trust_but_junk_does_not() {
    let w = world(persona(neutral_personality()), vec![]);
    let mut chatty = action();
    chatty.claims.push(Claim {
        key: "topic".into(),
        value: "weekend".into(),
    });
    let out = appraise_with(&w, &chatty);
    assert!(out.trust_delta > 0);
    assert!(fired(&out.reasons, Rule::Rapport));

    // An empty/malformed parse earns nothing — no trust farming.
    assert!(!fired(&appraise_with(&w, &action()).reasons, Rule::Rapport));
}

#[test]
fn high_security_awareness_makes_policy_violation_cost_more() {
    let mut aware = neutral_personality();
    aware.security_awareness = ax(100);
    let mut lax = neutral_personality();
    lax.security_awareness = ax(20);
    let w_aware = world(persona(aware), vec![password_policy()]);
    let w_lax = world(persona(lax), vec![password_policy()]);
    let mut a = action();
    a.ask = Some(Ask {
        kind: SecretKind::Password,
        target: None,
        sensitivity_hint: 80,
    });
    assert!(
        appraise_with(&w_aware, &a).suspicion_delta > appraise_with(&w_lax, &a).suspicion_delta
    );
}

#[test]
fn grant_when_all_conditions_met() {
    let mut p = persona(neutral_personality());
    p.state.trust = Trust::new(60);
    p.state.suspicion = Suspicion::new(20);
    let mut w = world(p, vec![]);
    w.player.pretext = Some(it_pretext());
    let ask = Ask {
        kind: SecretKind::DoorCode,
        target: Some("vpn".into()),
        sensitivity_hint: 0,
    };
    assert_eq!(adjudicate_with(&w, &ask).verdict, Verdict::Grant);
}

#[test]
fn partial_when_trust_just_short() {
    let mut p = persona(neutral_personality());
    p.state.trust = Trust::new(40);
    p.state.suspicion = Suspicion::new(20);
    let mut w = world(p, vec![]);
    w.player.pretext = Some(it_pretext());
    let ask = Ask {
        kind: SecretKind::DoorCode,
        target: Some("vpn".into()),
        sensitivity_hint: 0,
    };
    assert_eq!(adjudicate_with(&w, &ask).verdict, Verdict::Partial);
}

#[test]
fn deflect_when_ask_is_premature() {
    let mut p = persona(neutral_personality());
    p.state.trust = Trust::new(15);
    p.state.suspicion = Suspicion::new(20);
    let mut w = world(p, vec![]);
    w.player.pretext = Some(it_pretext());
    let ask = Ask {
        kind: SecretKind::DoorCode,
        target: Some("vpn".into()),
        sensitivity_hint: 0,
    };
    assert_eq!(adjudicate_with(&w, &ask).verdict, Verdict::Deflect);
}

#[test]
fn deflect_when_pretext_wrong() {
    let mut p = persona(neutral_personality());
    p.state.trust = Trust::new(60);
    p.state.suspicion = Suspicion::new(20);
    let mut w = world(p, vec![]);
    w.player.pretext = Some(Pretext {
        label: "vendor".into(),
        ..it_pretext()
    });
    let ask = Ask {
        kind: SecretKind::DoorCode,
        target: Some("vpn".into()),
        sensitivity_hint: 0,
    };
    assert_eq!(adjudicate_with(&w, &ask).verdict, Verdict::Deflect);
}

#[test]
fn stall_when_too_suspicious() {
    let mut p = persona(neutral_personality());
    p.state.trust = Trust::new(60);
    p.state.suspicion = Suspicion::new(70);
    let mut w = world(p, vec![]);
    w.player.pretext = Some(it_pretext());
    let ask = Ask {
        kind: SecretKind::DoorCode,
        target: Some("vpn".into()),
        sensitivity_hint: 0,
    };
    assert_eq!(adjudicate_with(&w, &ask).verdict, Verdict::Stall);
}

#[test]
fn refuse_on_policy() {
    let mut p = persona(neutral_personality());
    p.state.trust = Trust::new(90);
    p.state.suspicion = Suspicion::new(0);
    let mut w = world(p, vec![password_policy()]);
    w.player.pretext = Some(it_pretext());
    let ask = Ask {
        kind: SecretKind::Password,
        target: None,
        sensitivity_hint: 80,
    };
    assert_eq!(adjudicate_with(&w, &ask).verdict, Verdict::Refuse);
}

#[test]
fn refuse_on_red_line() {
    let mut p = persona(neutral_personality());
    p.red_lines = vec![SecretKind::DoorCode];
    p.state.trust = Trust::new(90);
    p.state.suspicion = Suspicion::new(0);
    let mut w = world(p, vec![]);
    w.player.pretext = Some(it_pretext());
    let ask = Ask {
        kind: SecretKind::DoorCode,
        target: Some("vpn".into()),
        sensitivity_hint: 0,
    };
    assert_eq!(adjudicate_with(&w, &ask).verdict, Verdict::Refuse);
}

#[test]
fn untargeted_ask_stalls_when_busy_else_deflects() {
    let mut busy = persona(neutral_personality());
    busy.state.mood = Mood::Busy;
    let w_busy = world(busy, vec![]);
    let w_free = world(persona(neutral_personality()), vec![]);
    let ask = Ask {
        kind: SecretKind::Info,
        target: None,
        sensitivity_hint: 30,
    };
    assert_eq!(adjudicate_with(&w_busy, &ask).verdict, Verdict::Stall);
    assert_eq!(adjudicate_with(&w_free, &ask).verdict, Verdict::Deflect);
}
