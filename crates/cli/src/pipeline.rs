use llm::{build_performer_prompt, fallback_line, heuristic_parse, OllamaClient, PerformerInput};
use world::{
    Appraisal, Ask, CausalEntry, ChannelKind, Message, ParsedAction, PersonaId, RunStatus,
    SalientFact, Sender, SuccessCondition, Verdict, World,
};

pub struct Engine {
    pub client: OllamaClient,
    pub online: bool,
    pub debug: bool,
}

pub struct TurnReport {
    pub speaker: String,
    pub reply: String,
    pub suspicion_delta: i16,
    pub trust_delta: i16,
    pub verdict: Option<Verdict>,
    pub reasons: Vec<CausalEntry>,
    pub status: RunStatus,
    /// A one-line LLM error (auth / not running / malformed Analyst output) for this turn.
    pub notice: Option<String>,
    /// Raw Analyst response, carried only under `--debug`.
    pub analyst_raw: Option<String>,
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
    let mut notice = None;
    let mut analyst_raw = None;

    // 1. Perceive — record the message, parse intent (Analyst, or heuristic offline).
    push_message(world, turn, Sender::Player, body.to_owned());
    let mut action = if engine.online {
        match engine
            .client
            .analyze(llm::analyst_system_prompt(), &analyst_user(body))
            .await
        {
            Ok(analysis) => {
                if engine.debug {
                    analyst_raw = Some(analysis.raw);
                } else if !analysis.parsed {
                    notice = Some(
                        "Analyst output wasn't valid JSON — rode inert fallback (try a model \
                         that honors structured output, or --debug to see the raw reply)"
                            .to_owned(),
                    );
                }
                analysis.action
            }
            Err(e) => {
                notice = Some(e.to_string());
                heuristic_parse(body)
            }
        }
    } else {
        heuristic_parse(body)
    };
    // 2-3. Appraise + adjudicate — the pure referee core, with its state committed.
    let StepOutcome {
        appraisal,
        verdict,
        verdict_reasons,
    } = referee_step(world, contact, &mut action, turn);

    // 4. Generate — Performer writes the reply, handed the verdict as a constraint and
    // the protected value only on Grant.
    let granted_value = grant_value(world, &action, verdict);
    let reply = generate(
        world,
        engine,
        contact,
        verdict,
        granted_value.as_deref(),
        &mut notice,
    )
    .await;

    // 5. Commit — persist reply, causal log, grant/detection effects, advance the clock.
    let mut reasons = appraisal.reasons;
    reasons.extend(verdict_reasons);
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
        notice,
        analyst_raw,
    }
}

/// The pure referee core of a turn: resolve the ask, appraise, commit the resulting
/// state (deltas, principle history, salient facts, established-claim and verification
/// flags), then adjudicate any ask. No I/O, no model — the part a balance test can drive.
pub struct StepOutcome {
    pub appraisal: Appraisal,
    pub verdict: Option<Verdict>,
    pub verdict_reasons: Vec<CausalEntry>,
}

pub fn referee_step(
    world: &mut World,
    contact: &PersonaId,
    action: &mut ParsedAction,
    turn: u32,
) -> StepOutcome {
    resolve_ask_target(world, contact, action);

    let appraisal = {
        let persona = world.persona(contact).expect("contact exists");
        referee::appraise(world, persona, action)
    };
    commit_appraisal(world, contact, turn, action, &appraisal);

    let (verdict, verdict_reasons) = match action.ask.as_ref() {
        Some(ask) => {
            let persona = world.persona(contact).expect("contact exists");
            let adj = referee::adjudicate(world, persona, ask);
            (Some(adj.verdict), adj.reasons)
        }
        None => (None, Vec::new()),
    };

    StepOutcome {
        appraisal,
        verdict,
        verdict_reasons,
    }
}

fn analyst_user(body: &str) -> String {
    format!("Message from the contact: \"{body}\"")
}

/// Match the Analyst's ask against a concrete secret the contact owns. Referent
/// (label/aliases) is the primary signal; kind is a fallback when referent is absent
/// and exactly one owned secret shares that kind.
pub fn resolve_ask_target(world: &World, contact: &PersonaId, action: &mut ParsedAction) {
    let Some(ask) = action.ask.as_mut() else {
        return;
    };
    if ask.target.is_some() {
        return;
    }
    ask.target = match_ask_to_secret(world, contact, ask);
}

fn match_ask_to_secret(world: &World, owner: &PersonaId, ask: &Ask) -> Option<world::SecretId> {
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

    // Secondary: kind match only when unambiguous.
    let kind_matches: Vec<_> = candidates.iter().filter(|s| s.kind == ask.kind).collect();
    if kind_matches.len() == 1 {
        return Some(kind_matches[0].id.clone());
    }
    None
}

fn normalize_phrase(s: &str) -> String {
    s.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn commit_appraisal(
    world: &mut World,
    contact: &PersonaId,
    turn: u32,
    action: &ParsedAction,
    appraisal: &Appraisal,
) {
    // The verification check (in `appraise`) already read the *pre-turn* flag; resolve
    // checkability here, before the mutable borrow, so we can latch it afterwards.
    let newly_verified = action
        .verification
        .as_deref()
        .is_some_and(|reference| world.org.verifiable_refs.iter().any(|r| r == reference));

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

    // Latch the concept, not the wording: once an authority claim is on the record, the
    // one-time scrutiny in `appraise` doesn't re-fire even as the Analyst paraphrases it.
    if action.authority_claim.is_some() {
        persona.state.beliefs.authority_claimed = true;
    }
    if newly_verified {
        persona.state.beliefs.authority_verified = true;
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
    notice: &mut Option<String>,
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
    match engine.client.perform(&prompt).await {
        Ok(text) => text,
        Err(e) => {
            notice.get_or_insert(e.to_string());
            fallback_line(verdict, persona)
        }
    }
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
