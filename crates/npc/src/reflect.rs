use world::{Memory, Reflection};

pub fn due_for_reflection(memory: &Memory, turn: u32, interval: u32) -> bool {
    if interval == 0 {
        return false;
    }
    let anchor = memory.reflections.last().map_or(0, |r| r.turn);
    turn.saturating_sub(anchor) >= interval
}

pub fn record_reflection(memory: &mut Memory, turn: u32, summary: String) {
    memory.reflections.push(Reflection { turn, summary });
}

pub fn reflection_context(memory: &Memory, max_lines: usize) -> String {
    let mut out = String::new();
    for obs in memory.observations.iter().rev().take(max_lines).rev() {
        out.push_str(&obs.body);
        out.push('\n');
    }
    for fact in memory.salient_facts.iter().rev().take(max_lines / 2) {
        out.push_str(&format!("{}: {}\n", fact.key, fact.value));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::Memory;

    #[test]
    fn reflection_fires_on_schedule() {
        let memory = Memory::default();
        assert!(!due_for_reflection(&memory, 4, 5));
        assert!(due_for_reflection(&memory, 5, 5));
    }

    #[test]
    fn reflection_resets_after_one_is_recorded() {
        let mut memory = Memory::default();
        record_reflection(&mut memory, 5, "feels off".into());
        assert!(!due_for_reflection(&memory, 9, 5));
        assert!(due_for_reflection(&memory, 10, 5));
    }
}
