use world::{Memory, Tuning};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievedMemory {
    pub lines: Vec<String>,
}

pub fn retrieve(memory: &Memory, query: &str, turn: u32, tuning: &Tuning) -> RetrievedMemory {
    let query_words = token_set(query);
    let mut scored: Vec<(i32, u32, String)> = Vec::new();

    for obs in &memory.observations {
        let text = obs.body.clone();
        let score = score_item(&query_words, &text, turn, obs.turn);
        if score > 0 {
            scored.push((score, obs.turn, text));
        }
    }
    for fact in &memory.salient_facts {
        let text = format!("{}: {}", fact.key, fact.value);
        let score = score_item(&query_words, &text, turn, fact.turn);
        if score > 0 {
            scored.push((score, fact.turn, text));
        }
    }
    for reflection in &memory.reflections {
        let score = score_item(&query_words, &reflection.summary, turn, reflection.turn);
        scored.push((score.max(1), reflection.turn, reflection.summary.clone()));
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));

    let mut lines = Vec::new();
    let mut chars = 0usize;
    for (_, _, text) in scored.into_iter().take(tuning.retrieval_max_items) {
        let next = chars + text.len() + 1;
        if next > tuning.retrieval_max_chars && !lines.is_empty() {
            break;
        }
        chars = next;
        lines.push(text);
        if lines.len() >= tuning.retrieval_max_items {
            break;
        }
    }

    RetrievedMemory { lines }
}

fn score_item(
    query_words: &std::collections::HashSet<String>,
    text: &str,
    turn: u32,
    item_turn: u32,
) -> i32 {
    let overlap = token_set(text)
        .iter()
        .filter(|w| query_words.contains(*w))
        .count() as i32;
    let age = turn.saturating_sub(item_turn).min(10);
    let recency = i32::try_from(10_u32.saturating_sub(age)).unwrap_or(0);
    overlap * 10 + recency
}

fn token_set(text: &str) -> std::collections::HashSet<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::{ChannelKind, Memory, SalientFact, Sender, Tuning};

    #[test]
    fn retrieval_returns_relevant_not_all() {
        let mut memory = Memory::default();
        for (turn, body) in [
            (1, "hey, how is the quarter-end close going"),
            (2, "need the VPN enrollment code for your laptop"),
            (3, "thanks for the update on the budget"),
        ] {
            memory.observations.push(world::Observation {
                turn,
                channel: ChannelKind::Messenger,
                sender: Sender::Player,
                body: body.to_owned(),
            });
        }
        memory.salient_facts.push(SalientFact {
            key: "topic".into(),
            value: "VPN enrollment".into(),
            turn: 2,
        });

        let out = retrieve(
            &memory,
            "can you send the code",
            4,
            &Tuning {
                retrieval_max_items: 2,
                ..Tuning::default()
            },
        );

        assert!(!out.lines.is_empty());
        assert!(out.lines.len() <= 2);
        assert!(out
            .lines
            .iter()
            .any(|l| l.contains("VPN") || l.contains("enrollment")));
    }

    #[test]
    fn retrieval_respects_char_budget() {
        let mut memory = Memory::default();
        for turn in 1..=10 {
            memory.observations.push(world::Observation {
                turn,
                channel: ChannelKind::Messenger,
                sender: Sender::Player,
                body: format!("message about enrollment code number {turn} with extra words"),
            });
        }
        let tuning = Tuning {
            retrieval_max_items: 10,
            retrieval_max_chars: 80,
            ..Tuning::default()
        };
        let out = retrieve(&memory, "enrollment code", 10, &tuning);
        let total: usize = out.lines.iter().map(String::len).sum();
        assert!(total <= tuning.retrieval_max_chars + 50);
        assert!(!out.lines.is_empty());
    }
}
