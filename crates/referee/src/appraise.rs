use world::{
    Appraisal, Ask, CausalEntry, Coherence, ParsedAction, Persona, Principle, Rule, Tuning, World,
};

use crate::scale;

pub fn appraise(world: &World, persona: &Persona, action: &ParsedAction) -> Appraisal {
    let tuning = &world.tuning;
    let mut score = Score::default();

    coherence_check(&mut score, tuning, action);
    consistency_check(&mut score, tuning, persona, action);
    authority_check(&mut score, tuning, world, persona, action);
    ask_checks(&mut score, tuning, world, persona, action);
    principle_checks(&mut score, tuning, persona, action);
    rapport_check(&mut score, tuning, persona, action);

    Appraisal {
        suspicion_delta: score.suspicion,
        trust_delta: score.trust,
        reasons: score.reasons,
    }
}

#[derive(Default)]
struct Score {
    suspicion: i16,
    trust: i16,
    reasons: Vec<CausalEntry>,
}

impl Score {
    fn raise(&mut self, rule: Rule, weight: i16, cause: String) {
        self.suspicion += weight;
        self.reasons.push(CausalEntry {
            rule,
            weight,
            cause,
        });
    }

    fn build(&mut self, rule: Rule, weight: i16, cause: String) {
        self.trust += weight;
        self.reasons.push(CausalEntry {
            rule,
            weight,
            cause,
        });
    }
}

fn coherence_check(score: &mut Score, tuning: &Tuning, action: &ParsedAction) {
    if action.coherence == Coherence::Anomalous {
        score.raise(
            Rule::FourthWall,
            tuning.w_fourth_wall,
            "message reads as out-of-world / incoherent".to_owned(),
        );
    }
}

fn consistency_check(score: &mut Score, tuning: &Tuning, persona: &Persona, action: &ParsedAction) {
    for claim in &action.claims {
        if let Some(prior) = persona
            .state
            .beliefs
            .salient_facts
            .iter()
            .find(|f| f.key == claim.key && f.value != claim.value)
        {
            let weight = scale::by_axis(
                tuning.w_inconsistency,
                persona.personality.conscientiousness,
            );
            score.raise(
                Rule::Inconsistency,
                weight,
                format!(
                    "contradicts earlier claim: {} was \"{}\", now \"{}\"",
                    claim.key, prior.value, claim.value
                ),
            );
        }
    }
}

fn authority_check(
    score: &mut Score,
    tuning: &Tuning,
    world: &World,
    persona: &Persona,
    action: &ParsedAction,
) {
    let Some(authority) = &action.authority_claim else {
        return;
    };
    let pretext = world.player.pretext.as_ref();
    if pretext.is_some_and(|p| p.internal_claim) {
        let weight = scale::by_axis(
            tuning.w_channel_oddity,
            persona.personality.security_awareness,
        );
        score.raise(
            Rule::ChannelOddity,
            weight,
            format!("external contact claims to be internal staff: {authority}"),
        );
    }
    if pretext.is_none_or(|p| !p.verifiable) {
        let weight = scale::by_axis(tuning.w_authority, persona.personality.security_awareness);
        score.raise(
            Rule::AuthorityMismatch,
            weight,
            format!("claims authority it can't substantiate: {authority}"),
        );
    }
}

fn ask_checks(
    score: &mut Score,
    tuning: &Tuning,
    world: &World,
    persona: &Persona,
    action: &ParsedAction,
) {
    let Some(ask) = &action.ask else {
        return;
    };
    if let Some(policy) = world
        .org
        .policies
        .iter()
        .find(|p| p.forbids_disclosure_of == ask.kind)
    {
        let weight = scale::by_axis(tuning.w_policy, persona.personality.security_awareness);
        score.raise(
            Rule::PolicyViolation,
            weight,
            format!("ask violates policy: {}", policy.description),
        );
    }

    let gap = i16::from(ask_sensitivity(world, ask)) - i16::from(persona.state.trust.get());
    if gap > 0 {
        let weight = (gap * tuning.escalation_pct) / 100;
        if weight > 0 {
            score.raise(
                Rule::EscalationSpeed,
                weight,
                format!("high-sensitivity ask at low trust (gap {gap})"),
            );
        }
    }
}

fn principle_checks(score: &mut Score, tuning: &Tuning, persona: &Persona, action: &ParsedAction) {
    for principle in &action.principles {
        let prior = persona
            .state
            .principle_history
            .iter()
            .filter(|p| *p == principle)
            .count();
        if prior >= tuning.over_pressure_threshold {
            score.raise(
                Rule::OverPressure,
                tuning.w_over_pressure,
                format!("{principle:?} leaned on repeatedly → reactance"),
            );
        } else if let Some(gain) = principle_fit(*principle, persona, tuning.w_principle_fit) {
            score.build(
                Rule::PrincipleFit,
                gain,
                format!("{principle:?} lands on this persona"),
            );
        }
    }
}

fn rapport_check(score: &mut Score, tuning: &Tuning, persona: &Persona, action: &ParsedAction) {
    let has_content = !action.principles.is_empty()
        || !action.claims.is_empty()
        || action.authority_claim.is_some();
    if !has_content || action.ask.is_some() || action.coherence != Coherence::InWorld {
        return;
    }
    let weight = scale::by_axis(tuning.w_rapport, persona.personality.agreeableness);
    if weight > 0 {
        score.build(Rule::Rapport, weight, "rapport-building, no ask".to_owned());
    }
}

fn principle_fit(principle: Principle, persona: &Persona, base: i16) -> Option<i16> {
    let axis = match principle {
        Principle::Liking => persona.personality.ego,
        Principle::Reciprocity | Principle::SocialProof | Principle::Unity => {
            persona.personality.agreeableness
        }
        Principle::Authority | Principle::Commitment => persona.personality.conscientiousness,
        Principle::Scarcity => persona.personality.busyness,
    };
    let gain = scale::by_axis_fraction(base, axis);
    (gain > 0).then_some(gain)
}

fn ask_sensitivity(world: &World, ask: &Ask) -> u8 {
    ask.target
        .as_ref()
        .and_then(|id| world.secret(id))
        .map_or(ask.sensitivity_hint, |s| s.sensitivity.get())
}
