#[cfg(test)]
mod tests {
    use world::{ChannelKind, Memory, Observation, Reflection, SalientFact, Sender};

    #[test]
    fn memory_survives_world_clone() {
        let mut memory = Memory::default();
        memory.observations.push(Observation {
            turn: 1,
            channel: ChannelKind::Messenger,
            sender: Sender::Player,
            body: "VPN enrollment code please".into(),
        });
        memory.salient_facts.push(SalientFact {
            key: "office".into(),
            value: "Houston".into(),
            turn: 1,
        });
        memory.focus = Some("priya_vpn_code".into());
        memory.focus_turn = Some(1);
        memory.reflections.push(Reflection {
            turn: 5,
            summary: "keeps pushing about access".into(),
        });

        let snapshot = memory.clone();
        let again = snapshot.clone();
        assert_eq!(snapshot, again);
    }
}
