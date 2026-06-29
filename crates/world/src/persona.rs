use crate::action::{Principle, SecretKind};
use crate::ids::{FactId, PersonaId, SecretId};
use crate::units::{Axis, Suspicion, Trust};

#[derive(Debug, Clone)]
pub struct Persona {
    pub id: PersonaId,
    pub name: String,
    pub title: String,
    pub department: String,
    pub reports_to: Option<PersonaId>,
    pub personality: Personality,
    pub voice: Voice,
    pub knowledge: Vec<SecretId>,
    pub facts: Vec<FactId>,
    pub relationships: Vec<Relationship>,
    pub hooks: Vec<String>,
    pub red_lines: Vec<SecretKind>,
    pub state: PersonaState,
}

#[derive(Debug, Clone, Copy)]
pub struct Personality {
    pub agreeableness: Axis,
    pub conscientiousness: Axis,
    pub security_awareness: Axis,
    pub busyness: Axis,
    pub ego: Axis,
    pub helpfulness: Axis,
}

#[derive(Debug, Clone)]
pub struct Voice {
    pub style: String,
    pub formality: Formality,
    pub avg_words: u16,
    pub quirks: Vec<String>,
    pub emoji: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formality {
    Casual,
    Neutral,
    Formal,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub other: PersonaId,
    pub trust: Trust,
    pub would_warn: bool,
    pub defers_to: bool,
}

#[derive(Debug, Clone)]
pub struct PersonaState {
    pub suspicion: Suspicion,
    pub trust: Trust,
    pub mood: Mood,
    pub beliefs: Beliefs,
    /// Principles this contact has leaned on, in order. Feeds reactance scoring.
    pub principle_history: Vec<Principle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Friendly,
    Neutral,
    Busy,
    Annoyed,
    Away,
}

#[derive(Debug, Clone, Default)]
pub struct Beliefs {
    pub accepted_pretext: Option<String>,
    pub salient_facts: Vec<SalientFact>,
}

#[derive(Debug, Clone)]
pub struct SalientFact {
    pub key: String,
    pub value: String,
    pub turn: u32,
}
