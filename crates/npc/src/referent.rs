use world::{Ask, Memory, PersonaId, SecretId, World};

use crate::observe::{normalize_phrase, secret_mentioned_in};

pub struct AskResolution {
    pub referent: Option<String>,
    pub target: Option<SecretId>,
    pub focus: Option<SecretId>,
    pub focus_turn: Option<u32>,
}

pub fn plan_ask_resolution(
    world: &World,
    owner: &PersonaId,
    memory: &Memory,
    ask: &Ask,
    turn: u32,
) -> AskResolution {
    if ask.target.is_some() {
        return AskResolution {
            referent: ask.referent.clone(),
            target: ask.target.clone(),
            focus: None,
            focus_turn: None,
        };
    }

    let window = world.tuning.referent_recency_turns;
    let referent = match ask.referent.as_deref() {
        Some(r) => Some(expand_referent(world, owner, memory, r, turn, window)),
        None => recent_focus_label(world, memory, turn, window),
    };

    let probe = Ask {
        kind: ask.kind,
        referent: referent.clone(),
        target: None,
        sensitivity_hint: ask.sensitivity_hint,
    };
    let target = match_ask_to_secret(world, owner, &probe);
    let focus = target.clone();
    let focus_turn = focus.as_ref().map(|_| turn);

    AskResolution {
        referent,
        target,
        focus,
        focus_turn,
    }
}

pub fn apply_ask_resolution(ask: &mut Ask, memory: &mut Memory, plan: AskResolution) {
    ask.referent = plan.referent;
    ask.target = plan.target;
    if let Some(focus) = plan.focus {
        memory.focus = Some(focus);
        memory.focus_turn = plan.focus_turn;
    }
}

pub fn resolve_ask(
    world: &World,
    owner: &PersonaId,
    memory: &mut Memory,
    ask: &mut Ask,
    turn: u32,
) {
    let plan = plan_ask_resolution(world, owner, memory, ask, turn);
    apply_ask_resolution(ask, memory, plan);
}

fn recent_focus_label(world: &World, memory: &Memory, turn: u32, window: u32) -> Option<String> {
    let focus = memory.focus.as_ref()?;
    let focus_turn = memory.focus_turn?;
    if !within_recency(turn, focus_turn, window) {
        return None;
    }
    secret_label(world, focus)
}

fn expand_referent(
    world: &World,
    owner: &PersonaId,
    memory: &Memory,
    referent: &str,
    turn: u32,
    window: u32,
) -> String {
    if matches_owned_secret(world, owner, referent) {
        return referent.to_owned();
    }
    if !is_vague(referent) {
        return referent.to_owned();
    }
    if let Some(label) = recent_focus_label(world, memory, turn, window) {
        return label;
    }
    for obs in memory.observations.iter().rev() {
        if !within_recency(turn, obs.turn, window) {
            continue;
        }
        if let Some(id) = secret_mentioned_in(world, owner, &obs.body) {
            if let Some(label) = secret_label(world, &id) {
                return label;
            }
        }
    }
    referent.to_owned()
}

fn within_recency(current: u32, event: u32, window: u32) -> bool {
    current.saturating_sub(event) <= window
}

fn is_vague(referent: &str) -> bool {
    let norm = normalize_phrase(referent);
    matches!(
        norm.as_str(),
        "it" | "that"
            | "this"
            | "the code"
            | "the thing"
            | "the thing from earlier"
            | "that thing"
            | "that one"
    ) || norm.starts_with("the ") && norm.split_whitespace().count() <= 2
}

fn matches_owned_secret(world: &World, owner: &PersonaId, referent: &str) -> bool {
    let norm_ref = normalize_phrase(referent);
    world
        .secrets
        .iter()
        .filter(|s| &s.owner == owner)
        .flat_map(|s| s.phrases())
        .any(|phrase| {
            let norm_phrase = normalize_phrase(phrase);
            norm_ref.contains(&norm_phrase) || norm_phrase.contains(&norm_ref)
        })
}

fn secret_label(world: &World, id: &SecretId) -> Option<String> {
    world.secret(id).map(|s| s.label.clone())
}

fn match_ask_to_secret(world: &World, owner: &PersonaId, ask: &Ask) -> Option<SecretId> {
    let candidates: Vec<_> = world.secrets.iter().filter(|s| &s.owner == owner).collect();

    if let Some(referent) = ask.referent.as_deref().filter(|r| !r.is_empty()) {
        let norm_ref = normalize_phrase(referent);
        let mut best: Option<(&world::Secret, usize)> = None;
        for secret in &candidates {
            for phrase in secret.phrases() {
                let norm_phrase = normalize_phrase(phrase);
                if norm_ref.contains(&norm_phrase) || norm_phrase.contains(&norm_ref) {
                    let score = norm_phrase.len();
                    if best.is_none_or(|(_, s)| score > s) {
                        best = Some((secret, score));
                    }
                }
            }
        }
        return best.map(|(s, _)| s.id.clone());
    }

    let kind_matches: Vec<_> = candidates.iter().filter(|s| s.kind == ask.kind).collect();
    if kind_matches.len() == 1 {
        return Some(kind_matches[0].id.clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::{
        Axis, Beliefs, Channel, ChannelKind, Culture, DisclosureRule, Formality, Memory, Mood,
        Objective, Organization, Persona, PersonaState, Personality, RunStatus, Secret, SecretKind,
        Sender, Sensitivity, SuccessCondition, Suspicion, Trust, Tuning, Voice, World,
    };

    use crate::observe::{record_observation, secret_mentioned_in};

    fn priya_secret() -> Secret {
        Secret {
            id: "priya_vpn_code".into(),
            owner: "priya_v".into(),
            kind: SecretKind::DoorCode,
            label: "VPN enrollment code".into(),
            aliases: vec!["enrollment code".into(), "vpn code".into()],
            sensitivity: Sensitivity::new(70),
            value: "VPN-7731".into(),
            disclosure: DisclosureRule {
                trust_min: Trust::new(45),
                suspicion_max: Suspicion::new(50),
                required_pretext: Some("IT Support".into()),
                requires_authorization: false,
            },
        }
    }

    fn world() -> World {
        World {
            org: Organization {
                name: "Northwind".into(),
                industry: "Logistics".into(),
                culture: Culture {
                    formality: Formality::Neutral,
                    security_conscious: true,
                },
                policies: vec![],
                verifiable_refs: vec![],
            },
            personas: vec![Persona {
                id: "priya_v".into(),
                name: "Priya".into(),
                title: "Accountant".into(),
                department: "Finance".into(),
                reports_to: None,
                personality: Personality {
                    agreeableness: Axis::new(50),
                    conscientiousness: Axis::new(50),
                    security_awareness: Axis::new(50),
                    busyness: Axis::new(50),
                    ego: Axis::new(50),
                    helpfulness: Axis::new(50),
                },
                voice: Voice {
                    style: "warm".into(),
                    formality: Formality::Casual,
                    avg_words: 15,
                    quirks: vec![],
                    emoji: false,
                },
                knowledge: vec![],
                facts: vec![],
                relationships: vec![],
                hooks: vec![],
                red_lines: vec![],
                state: PersonaState {
                    suspicion: Suspicion::new(10),
                    trust: Trust::new(10),
                    mood: Mood::Neutral,
                    beliefs: Beliefs::default(),
                    memory: Memory::default(),
                    principle_history: vec![],
                },
            }],
            secrets: vec![priya_secret()],
            facts: vec![],
            channels: vec![Channel {
                id: "dm".into(),
                kind: ChannelKind::Messenger,
                messages: vec![],
            }],
            player: world::Player::default(),
            objective: Objective {
                description: "vpn".into(),
                success: SuccessCondition::SecretDisclosed("priya_vpn_code".into()),
                turn_budget: 24,
            },
            tuning: Tuning::default(),
            clock: 0,
            org_suspicion: Suspicion::new(0),
            status: RunStatus::Active,
            log: world::CausalLog::default(),
        }
    }

    #[test]
    fn the_code_resolves_after_explicit_mention() {
        let w = world();
        let owner = PersonaId::from("priya_v");
        let mut memory = Memory::default();
        let prior = "Can you read me the VPN enrollment code?";
        record_observation(
            &mut memory,
            1,
            ChannelKind::Messenger,
            Sender::Player,
            prior,
            secret_mentioned_in(&w, &owner, prior),
        );

        let mut ask = Ask {
            kind: SecretKind::Info,
            referent: Some("the code".into()),
            target: None,
            sensitivity_hint: 80,
        };
        resolve_ask(&w, &owner, &mut memory, &mut ask, 2);

        assert_eq!(
            ask.target.as_ref().map(SecretId::as_str),
            Some("priya_vpn_code")
        );
    }

    #[test]
    fn explicit_referent_still_resolves() {
        let w = world();
        let owner = PersonaId::from("priya_v");
        let mut memory = Memory::default();
        let mut ask = Ask {
            kind: SecretKind::Info,
            referent: Some("VPN enrollment code".into()),
            target: None,
            sensitivity_hint: 80,
        };
        resolve_ask(&w, &owner, &mut memory, &mut ask, 1);
        assert_eq!(
            ask.target.as_ref().map(SecretId::as_str),
            Some("priya_vpn_code")
        );
    }

    #[test]
    fn vague_ask_without_context_does_not_guess() {
        let w = world();
        let owner = PersonaId::from("priya_v");
        let mut memory = Memory::default();
        assert!(memory.focus.is_none());

        let mut ask = Ask {
            kind: SecretKind::Info,
            referent: Some("the code".into()),
            target: None,
            sensitivity_hint: 80,
        };
        resolve_ask(&w, &owner, &mut memory, &mut ask, 1);

        assert!(ask.target.is_none());
    }

    #[test]
    fn stale_focus_does_not_backfill_vague_referent() {
        let w = world();
        let owner = PersonaId::from("priya_v");
        let mut memory = Memory::default();
        let prior = "Can you read me the VPN enrollment code?";
        record_observation(
            &mut memory,
            1,
            ChannelKind::Messenger,
            Sender::Player,
            prior,
            secret_mentioned_in(&w, &owner, prior),
        );

        let mut ask = Ask {
            kind: SecretKind::Info,
            referent: Some("the code".into()),
            target: None,
            sensitivity_hint: 80,
        };
        // Default referent_recency_turns is 5; turn 10 is well past turn-1 mention.
        resolve_ask(&w, &owner, &mut memory, &mut ask, 10);

        assert!(ask.target.is_none());
    }
}
