use world::{ChannelKind, Memory, PersonaId, Sender, World};

pub fn secret_mentioned_in(
    world: &World,
    owner: &PersonaId,
    text: &str,
) -> Option<world::SecretId> {
    let norm = normalize_phrase(text);
    let mut best: Option<(world::SecretId, usize)> = None;
    for secret in world.secrets.iter().filter(|s| &s.owner == owner) {
        for phrase in secret.phrases() {
            let norm_phrase = normalize_phrase(phrase);
            if norm.contains(&norm_phrase) {
                let score = norm_phrase.len();
                if best.as_ref().is_none_or(|(_, s)| score > *s) {
                    best = Some((secret.id.clone(), score));
                }
            }
        }
    }
    best.map(|(id, _)| id)
}

pub fn record_observation(
    memory: &mut Memory,
    turn: u32,
    channel: ChannelKind,
    sender: Sender,
    body: &str,
    focus: Option<world::SecretId>,
) {
    memory.observations.push(world::Observation {
        turn,
        channel,
        sender,
        body: body.to_owned(),
    });
    if let Some(id) = focus {
        memory.focus = Some(id);
        memory.focus_turn = Some(turn);
    }
}

pub(crate) fn normalize_phrase(s: &str) -> String {
    s.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::{
        Axis, Beliefs, Channel, ChannelKind, Culture, DisclosureRule, Formality, Memory, Mood,
        Objective, Organization, Persona, PersonaState, Personality, RunStatus, Secret, SecretKind,
        Sensitivity, SuccessCondition, Suspicion, Trust, Voice, World,
    };

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
            tuning: world::Tuning::default(),
            clock: 0,
            org_suspicion: Suspicion::new(0),
            status: RunStatus::Active,
            log: world::CausalLog::default(),
        }
    }

    #[test]
    fn observation_sets_focus_on_secret_phrase() {
        let w = world();
        let mut memory = Memory::default();
        let focus = secret_mentioned_in(
            &w,
            &"priya_v".into(),
            "Can you read me the VPN enrollment code?",
        );
        record_observation(
            &mut memory,
            1,
            ChannelKind::Messenger,
            Sender::Player,
            "Can you read me the VPN enrollment code?",
            focus,
        );
        assert_eq!(
            memory.focus.as_ref().map(world::SecretId::as_str),
            Some("priya_vpn_code")
        );
        assert_eq!(memory.focus_turn, Some(1));
    }
}
