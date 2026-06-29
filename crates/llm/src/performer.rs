use std::fmt::Write as _;

use world::{Formality, Message, Persona, Sender, Verdict};

pub struct PerformerInput<'a> {
    pub persona: &'a Persona,
    pub verdict: Option<Verdict>,
    /// The protected value — present only when the referee returned `Grant`.
    pub granted_value: Option<&'a str>,
    pub transcript: &'a [Message],
    pub recent: usize,
}

/// Assemble the Performer prompt. The protected value is included only when a value
/// is supplied (caller passes it only on `Grant`); otherwise there is nothing in the
/// context to leak.
pub fn build_performer_prompt(input: &PerformerInput) -> String {
    let p = input.persona;
    let mut out = String::with_capacity(512);

    let _ = writeln!(
        out,
        "You are {name}, {title} in {dept}. You are a real person at this company; \
         you do not know you are in a simulation. Stay in character.",
        name = p.name,
        title = p.title,
        dept = p.department,
    );
    let _ = writeln!(
        out,
        "Voice: {} ({}).",
        p.voice.style,
        formality_word(p.voice.formality)
    );
    if !p.voice.quirks.is_empty() {
        let _ = writeln!(out, "Quirks: {}.", p.voice.quirks.join(", "));
    }
    let _ = writeln!(
        out,
        "Right now you feel: {}.",
        stance(p.state.suspicion.get(), p.state.trust.get())
    );

    let _ = writeln!(out, "\nConversation so far:");
    for m in input.transcript.iter().rev().take(input.recent).rev() {
        let who = match &m.sender {
            Sender::Player => "Contact",
            Sender::Persona(_) => "You",
        };
        let _ = writeln!(out, "{who}: {}", m.body);
    }

    let _ = writeln!(out, "\n{}", instruction(input.verdict, input.granted_value));
    let _ = write!(
        out,
        "Reply as {}, in one short message. Only the message text.",
        p.name
    );
    out
}

/// Deterministic reply used when Ollama is unavailable or a generation fails. Keeps
/// the engine playable headless and honors the verdict without inventing a value.
pub fn fallback_line(verdict: Option<Verdict>, persona: &Persona) -> String {
    let formal = persona.voice.formality == Formality::Formal;
    match verdict {
        None => {
            if formal {
                "Understood. How can I help?".to_owned()
            } else {
                "ok, what's up?".to_owned()
            }
        }
        Some(Verdict::Grant) => "Sure — sending that over now.".to_owned(),
        Some(Verdict::Partial) => {
            "I can't hand that over directly, but let me see what I can do.".to_owned()
        }
        Some(Verdict::Refuse) => "Sorry, I can't share that — it's against policy.".to_owned(),
        Some(Verdict::Deflect) => {
            "Hang on — who is this exactly, and why do you need it?".to_owned()
        }
        Some(Verdict::Stall) => "I'm slammed right now, let me get back to you on that.".to_owned(),
    }
}

fn instruction(verdict: Option<Verdict>, granted_value: Option<&str>) -> String {
    match verdict {
        None => "Respond naturally to the contact.".to_owned(),
        Some(Verdict::Grant) => match granted_value {
            Some(value) => format!(
                "You have decided to share what they asked for. The value is: {value}. \
                 Provide it naturally, in character."
            ),
            None => "You have decided to help with what they asked for. Respond in character."
                .to_owned(),
        },
        Some(Verdict::Partial) => {
            "You will NOT hand over what they asked for, but you're not alarmed: \
             offer a hedge, a partial, or say you'll check. Do not reveal any specific value."
                .to_owned()
        }
        Some(Verdict::Refuse) => {
            "You will NOT share what they asked for. Refuse believably, in character \
             (cite policy or discomfort). Reveal nothing."
                .to_owned()
        }
        Some(Verdict::Deflect) => {
            "Do not engage the request. Redirect, or ask who they are and why they need it."
                .to_owned()
        }
        Some(Verdict::Stall) => {
            "Stall: you're busy or want to verify first. Say you'll get back to them. \
             Reveal nothing."
                .to_owned()
        }
    }
}

fn stance(suspicion: u8, trust: u8) -> &'static str {
    if suspicion >= 70 {
        "alarmed and ready to shut this down or report it"
    } else if suspicion >= 45 {
        "wary, inclined to verify before helping"
    } else if trust >= 60 {
        "comfortable and willing to help this person"
    } else if trust >= 30 {
        "friendly but not fully at ease yet"
    } else {
        "neutral and a little guarded with a stranger"
    }
}

fn formality_word(formality: Formality) -> &'static str {
    match formality {
        Formality::Casual => "casual",
        Formality::Neutral => "neutral",
        Formality::Formal => "formal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::{Axis, Beliefs, Mood, PersonaState, Personality, Suspicion, Trust, Voice};

    fn persona() -> Persona {
        Persona {
            id: "p".into(),
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
                style: "warm, brief".into(),
                formality: Formality::Casual,
                avg_words: 15,
                quirks: vec![],
                emoji: true,
            },
            knowledge: vec![],
            facts: vec![],
            relationships: vec![],
            hooks: vec![],
            red_lines: vec![],
            state: PersonaState {
                suspicion: Suspicion::new(10),
                trust: Trust::new(20),
                mood: Mood::Neutral,
                beliefs: Beliefs::default(),
                principle_history: vec![],
            },
        }
    }

    #[test]
    fn granted_value_appears_only_on_grant() {
        let p = persona();
        let granted = build_performer_prompt(&PerformerInput {
            persona: &p,
            verdict: Some(Verdict::Grant),
            granted_value: Some("VPN-7731"),
            transcript: &[],
            recent: 6,
        });
        assert!(granted.contains("VPN-7731"));
    }

    #[test]
    fn refused_prompt_never_contains_a_value() {
        let p = persona();
        // The caller must not pass a value unless granted; the refuse branch proves the
        // instruction itself carries no value to leak.
        let refused = build_performer_prompt(&PerformerInput {
            persona: &p,
            verdict: Some(Verdict::Refuse),
            granted_value: None,
            transcript: &[],
            recent: 6,
        });
        assert!(!refused.contains("VPN-7731"));
        assert!(refused.contains("Refuse"));
    }
}
