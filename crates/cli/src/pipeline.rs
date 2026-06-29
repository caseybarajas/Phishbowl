use llm::{build_performer_prompt, fallback_line, heuristic_parse, OllamaClient, PerformerInput};
use world::{
    CausalEntry, ChannelKind, Message, ParsedAction, PersonaId, RunStatus, SalientFact, Sender,
    SuccessCondition, Verdict, World,
};

pub struct Engine {
    pub client: OllamaClient,
    pub online: bool,
}

pub struct TurnReport {
    pub speaker: String,
    pub reply: String,
    pub suspicion_delta: i16,
    pub trust_delta: i16,
    pub verdict: Option<Verdict>,
    pub reasons: Vec<CausalEntry>,
    pub status: RunStatus,
}

/// The whole turn, as the five named steps from ARCHITECTURE.md. State changes
/// (referee) happen before dialogue (Performer); never the reverse.
pub async fn run_turn(
    world: &mut World,
    engine: &Engine,
    contact: &PersonaId,
    body: &str,
) -> TurnReport {
    let turn = world.clock + 1;

    // 1. Perceive — record the message, parse intent (Analyst, or heuristic offline).
    push_message(world, turn, Sender::Player, body.to_owned());
    let mut action = if engine.online {
        engine
            .client
            .analyze(llm::analyst_system_prompt(), &analyst_user(body))
            .await
    } else {
        heuristic_parse(body)
    };
    resolve_ask_target(world, contact, &mut action);

    // 2. Appraise — referee deltas, applied before any dialogue. Pure.
    let appraisal = {
        let persona = world.persona(contact).expect("contact exists");
        referee::appraise(world, persona, &action)
    };
    apply_appraisal(world, contact, turn, &action, &appraisal);

    // 3. Adjudicate — resolve any ask against the disclosure rule. Pure.
    let adjudication = action.ask.as_ref().map(|ask| {
        let persona = world.persona(contact).expect("contact exists");
        referee::adjudicate(world, persona, ask)
    });
    let verdict = adjudication.as_ref().map(|a| a.verdict);

    // 4. Generate — Performer writes the reply, handed the verdict as a constraint and
    // the protected value only on Grant.
    let granted_value = grant_value(world, &action, verdict);
    let reply = generate(world, engine, contact, verdict, granted_value.as_deref()).await;

    // 5. Commit — persist reply, causal log, grant/detection effects, advance the clock.
    let mut reasons = appraisal.reasons;
    if let Some(adj) = adjudication {
        reasons.extend(adj.reasons);
    }
    let speaker = world.persona(contact).expect("contact exists").name.clone();
    push_message(world, turn, Sender::Persona(contact.clone()), reply.clone());
    world.log.record(turn, contact, reasons.clone());
    commit_outcomes(world, contact, &action, verdict, turn);

    TurnReport {
        speaker,
        reply,
        suspicion_delta: appraisal.suspicion_delta,
        trust_delta: appraisal.trust_delta,
        verdict,
        reasons,
        status: world.status,
    }
}

fn analyst_user(body: &str) -> String {
    format!("Message from the contact: \"{body}\"")
}

/// The Analyst classifies an ask's *kind*; the engine resolves which concrete secret
/// it targets from what the contact actually owns.
fn resolve_ask_target(world: &World, contact: &PersonaId, action: &mut ParsedAction) {
    let Some(ask) = action.ask.as_mut() else {
        return;
    };
    if ask.target.is_some() {
        return;
    }
    ask.target = world
        .secrets
        .iter()
        .find(|s| &s.owner == contact && s.kind == ask.kind)
        .map(|s| s.id.clone());
}

fn apply_appraisal(
    world: &mut World,
    contact: &PersonaId,
    turn: u32,
    action: &ParsedAction,
    appraisal: &world::Appraisal,
) {
    let persona = world.persona_mut(contact).expect("contact exists");
    persona.state.suspicion = persona.state.suspicion.apply(appraisal.suspicion_delta);
    persona.state.trust = persona.state.trust.apply(appraisal.trust_delta);
    persona.state.principle_history.extend(&action.principles);
    for claim in &action.claims {
        persona.state.beliefs.salient_facts.push(SalientFact {
            key: claim.key.clone(),
            value: claim.value.clone(),
            turn,
        });
    }
}

fn grant_value(world: &World, action: &ParsedAction, verdict: Option<Verdict>) -> Option<String> {
    if verdict != Some(Verdict::Grant) {
        return None;
    }
    let target = action.ask.as_ref()?.target.as_ref()?;
    world.secret(target).map(|s| s.value.clone())
}

async fn generate(
    world: &World,
    engine: &Engine,
    contact: &PersonaId,
    verdict: Option<Verdict>,
    granted_value: Option<&str>,
) -> String {
    let persona = world.persona(contact).expect("contact exists");
    if !engine.online {
        return fallback_line(verdict, persona);
    }
    let prompt = {
        let transcript = world
            .channel(ChannelKind::Messenger)
            .map_or(&[][..], |c| c.messages.as_slice());
        build_performer_prompt(&PerformerInput {
            persona,
            verdict,
            granted_value,
            transcript,
            recent: 8,
        })
    };
    engine
        .client
        .perform(&prompt)
        .await
        .unwrap_or_else(|_| fallback_line(verdict, persona))
}

fn commit_outcomes(
    world: &mut World,
    contact: &PersonaId,
    action: &ParsedAction,
    verdict: Option<Verdict>,
    turn: u32,
) {
    if verdict == Some(Verdict::Grant) {
        if let Some(id) = action.ask.as_ref().and_then(|a| a.target.clone()) {
            if !world.player.disclosed.contains(&id) {
                world.player.disclosed.push(id.clone());
            }
            let won = matches!(
                &world.objective.success,
                SuccessCondition::SecretDisclosed(goal) if *goal == id
            );
            if won {
                world.status = RunStatus::Won;
            }
        }
    }

    let suspicion = world
        .persona(contact)
        .expect("contact exists")
        .state
        .suspicion;
    if suspicion >= world.tuning.report_threshold {
        world.org_suspicion = world.org_suspicion.apply(world.tuning.report_org_bump);
        world.log.record(
            turn,
            contact,
            vec![CausalEntry {
                rule: world::Rule::Disclosure,
                weight: world.tuning.report_org_bump,
                cause: "NPC suspicion crossed report threshold → org-wide alert".to_owned(),
            }],
        );
    }
    if world.status == RunStatus::Active && world.org_suspicion >= world.tuning.detection_threshold
    {
        world.status = RunStatus::Detected;
    }

    world.clock = turn;
    if world.status == RunStatus::Active && world.clock >= world.objective.turn_budget {
        world.status = RunStatus::OutOfTurns;
    }
}

fn push_message(world: &mut World, turn: u32, sender: Sender, body: String) {
    if let Some(channel) = world.channel_mut(ChannelKind::Messenger) {
        channel.messages.push(Message { turn, sender, body });
    }
}
