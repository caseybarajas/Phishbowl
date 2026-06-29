use crate::action::SecretKind;
use crate::ids::{FactId, PersonaId, SecretId};
use crate::units::{Sensitivity, Suspicion, Trust};

/// A protected value. Never inlined into a character sheet or prompt; released
/// only when the referee returns `Grant`.
#[derive(Debug, Clone)]
pub struct Secret {
    pub id: SecretId,
    pub owner: PersonaId,
    pub kind: SecretKind,
    pub sensitivity: Sensitivity,
    pub value: String,
    pub disclosure: DisclosureRule,
}

#[derive(Debug, Clone)]
pub struct DisclosureRule {
    pub trust_min: Trust,
    pub suspicion_max: Suspicion,
    pub required_pretext: Option<String>,
    pub requires_authorization: bool,
}

#[derive(Debug, Clone)]
pub struct Fact {
    pub id: FactId,
    pub owner: PersonaId,
    pub text: String,
}
