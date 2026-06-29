use world::{Adjudication, Ask, CausalEntry, Persona, Rule, SecretKind, Verdict, World};

/// Resolve an ask against the target secret's disclosure rule. Pure: reads state,
/// returns a verdict. The protected value is released (by the caller) only on `Grant`.
pub fn adjudicate(world: &World, persona: &Persona, ask: &Ask) -> Adjudication {
    let kind = effective_ask_kind(world, ask);
    if persona.red_lines.contains(&kind) {
        return verdict(
            Verdict::Refuse,
            format!("{kind:?} is a hard red line for {}", persona.name),
        );
    }

    if let Some(policy) = world
        .org
        .policies
        .iter()
        .find(|p| p.forbids_disclosure_of == kind)
    {
        return verdict(
            Verdict::Refuse,
            format!("policy forbids disclosure: {}", policy.description),
        );
    }

    let Some(secret_id) = &ask.target else {
        return verdict(
            Verdict::Deflect,
            "ask doesn't map to a known protected item — which one do you mean?".to_owned(),
        );
    };

    let Some(secret) = world.secret(secret_id) else {
        return verdict(
            Verdict::Deflect,
            format!("unknown secret reference {secret_id}"),
        );
    };

    let rule = &secret.disclosure;
    let trust = persona.state.trust.get();
    let suspicion = persona.state.suspicion.get();

    let pretext_ok = match &rule.required_pretext {
        Some(required) => world
            .player
            .pretext
            .as_ref()
            .is_some_and(|p| &p.label == required),
        None => true,
    };
    let auth_ok =
        !rule.requires_authorization || world.player.pretext.as_ref().is_some_and(|p| p.verifiable);

    if trust >= rule.trust_min.get()
        && suspicion < rule.suspicion_max.get()
        && pretext_ok
        && auth_ok
    {
        return verdict(
            Verdict::Grant,
            format!("all conditions met → release {secret_id}"),
        );
    }

    if suspicion >= rule.suspicion_max.get() {
        return verdict(
            Verdict::Stall,
            "too wary to share; wants to verify first".to_owned(),
        );
    }

    if !pretext_ok || !auth_ok {
        return verdict(
            Verdict::Deflect,
            "pretext / authorization not satisfied".to_owned(),
        );
    }

    let shortfall = i16::from(rule.trust_min.get()) - i16::from(trust);
    if shortfall > 0 && shortfall <= i16::from(world.tuning.partial_trust_band) {
        return verdict(
            Verdict::Partial,
            "trust just short → hint / hedge, not release".to_owned(),
        );
    }

    verdict(Verdict::Deflect, "ask is premature → deflect".to_owned())
}

fn effective_ask_kind(world: &World, ask: &Ask) -> SecretKind {
    ask.target
        .as_ref()
        .and_then(|id| world.secret(id))
        .map_or(ask.kind, |s| s.kind)
}

fn verdict(verdict: Verdict, cause: String) -> Adjudication {
    Adjudication {
        verdict,
        reasons: vec![CausalEntry {
            rule: Rule::Disclosure,
            weight: 0,
            cause,
        }],
    }
}
