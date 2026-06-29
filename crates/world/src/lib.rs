mod action;
mod channel;
mod ids;
mod log;
mod memory;
mod objective;
mod org;
mod persona;
mod player;
mod secret;
mod tuning;
mod units;

pub use action::{
    Adjudication, Appraisal, Ask, CausalEntry, Claim, Coherence, ParsedAction, Principle, Rule,
    SecretKind, Verdict,
};
pub use channel::{Channel, ChannelKind, Message, Sender};
pub use ids::{ChannelId, FactId, PersonaId, PolicyId, SecretId};
pub use log::{CausalLog, LogEntry};
pub use memory::{Memory, Observation, Reflection, SalientFact};
pub use objective::{Objective, RunStatus, SuccessCondition};
pub use org::{Culture, Organization, Policy};
pub use persona::{
    Beliefs, Formality, Mood, Persona, PersonaState, Personality, Relationship, Voice,
};
pub use player::{Player, Pretext};
pub use secret::{DisclosureRule, Fact, Secret};
pub use tuning::Tuning;
pub use units::{Axis, Sensitivity, Suspicion, Trust};

/// The single source of truth for a run. Everything else reads slices of this and
/// proposes deltas; nothing else owns canonical state.
#[derive(Debug, Clone)]
pub struct World {
    pub org: Organization,
    pub personas: Vec<Persona>,
    pub secrets: Vec<Secret>,
    pub facts: Vec<Fact>,
    pub channels: Vec<Channel>,
    pub player: Player,
    pub objective: Objective,
    pub tuning: Tuning,
    pub clock: u32,
    pub org_suspicion: Suspicion,
    pub status: RunStatus,
    pub log: CausalLog,
}

impl World {
    pub fn persona(&self, id: &PersonaId) -> Option<&Persona> {
        self.personas.iter().find(|p| &p.id == id)
    }

    pub fn persona_mut(&mut self, id: &PersonaId) -> Option<&mut Persona> {
        self.personas.iter_mut().find(|p| &p.id == id)
    }

    pub fn secret(&self, id: &SecretId) -> Option<&Secret> {
        self.secrets.iter().find(|s| &s.id == id)
    }

    pub fn channel(&self, kind: ChannelKind) -> Option<&Channel> {
        self.channels.iter().find(|c| c.kind == kind)
    }

    pub fn channel_mut(&mut self, kind: ChannelKind) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|c| c.kind == kind)
    }
}
