use crate::channel::{ChannelKind, Sender};
use crate::ids::SecretId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Memory {
    pub observations: Vec<Observation>,
    pub salient_facts: Vec<SalientFact>,
    /// Last secret topic explicitly in play (vague referent backfill).
    pub focus: Option<SecretId>,
    /// Turn when `focus` was last set.
    pub focus_turn: Option<u32>,
    pub reflections: Vec<Reflection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reflection {
    pub turn: u32,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub turn: u32,
    pub channel: ChannelKind,
    pub sender: Sender,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalientFact {
    pub key: String,
    pub value: String,
    pub turn: u32,
}
