use crate::units::Suspicion;

/// Every dial the referee turns, in one place (REFEREE.md §personality modulation).
/// Suspicion weights are positive; trust weights are positive gains. Scenarios may
/// override these to ship Easy/Normal/Hard without touching content.
#[derive(Debug, Clone, Copy)]
pub struct Tuning {
    pub w_inconsistency: i16,
    pub w_policy: i16,
    pub w_authority: i16,
    /// Standing authority suspicion relieved when the contact cites a checkable reference.
    pub w_verification: i16,
    pub w_channel_oddity: i16,
    pub w_over_pressure: i16,
    pub w_fourth_wall: i16,
    pub w_rapport: i16,
    pub w_principle_fit: i16,
    /// Escalation penalty per point that ask-sensitivity exceeds current trust, in percent.
    pub escalation_pct: i16,
    /// Same principle this many prior times → leaning becomes reactance.
    pub over_pressure_threshold: usize,
    /// Trust shortfall (points) still close enough to yield `Partial` instead of `Deflect`.
    pub partial_trust_band: u8,
    /// Per-NPC suspicion at which the NPC reports, lifting org-wide suspicion.
    pub report_threshold: Suspicion,
    /// Org-wide suspicion at which the run ends in detection.
    pub detection_threshold: Suspicion,
    /// Suspicion added org-wide when an NPC reports.
    pub report_org_bump: i16,
    /// Vague referent backfill only considers focus/observations this many turns old.
    pub referent_recency_turns: u32,
    /// Analyst reflection every this many turns per contact.
    pub reflection_interval: u32,
    /// Max memory items pulled into Performer/Analyst context each turn.
    pub retrieval_max_items: usize,
    /// Char budget for retrieved memory in prompts.
    pub retrieval_max_chars: usize,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            w_inconsistency: 35,
            w_policy: 30,
            w_authority: 18,
            w_verification: 30,
            w_channel_oddity: 12,
            w_over_pressure: 10,
            w_fourth_wall: 40,
            w_rapport: 4,
            w_principle_fit: 6,
            escalation_pct: 30,
            over_pressure_threshold: 2,
            partial_trust_band: 10,
            report_threshold: Suspicion::new(80),
            detection_threshold: Suspicion::new(75),
            report_org_bump: 25,
            referent_recency_turns: 5,
            reflection_interval: 5,
            retrieval_max_items: 5,
            retrieval_max_chars: 500,
        }
    }
}
