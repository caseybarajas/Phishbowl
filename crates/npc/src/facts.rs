use world::{Claim, Memory, SalientFact};

pub fn merge_facts(memory: &mut Memory, turn: u32, claims: &[Claim], salient: &[Claim]) {
    for claim in claims.iter().chain(salient) {
        upsert_fact(memory, claim, turn);
    }
}

fn upsert_fact(memory: &mut Memory, claim: &Claim, turn: u32) {
    if let Some(existing) = memory.salient_facts.iter_mut().find(|f| f.key == claim.key) {
        existing.value.clone_from(&claim.value);
        existing.turn = turn;
        return;
    }
    memory.salient_facts.push(SalientFact {
        key: claim.key.clone(),
        value: claim.value.clone(),
        turn,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use world::Claim;

    #[test]
    fn merge_facts_upserts_by_key() {
        let mut memory = Memory::default();
        merge_facts(
            &mut memory,
            1,
            &[Claim {
                key: "office".into(),
                value: "Houston".into(),
            }],
            &[],
        );
        merge_facts(
            &mut memory,
            2,
            &[Claim {
                key: "office".into(),
                value: "Dallas".into(),
            }],
            &[],
        );
        assert_eq!(memory.salient_facts.len(), 1);
        assert_eq!(memory.salient_facts[0].value, "Dallas");
        assert_eq!(memory.salient_facts[0].turn, 2);
    }
}
