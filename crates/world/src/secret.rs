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
    /// Human-readable name for what this secret is (e.g. "VPN enrollment code").
    pub label: String,
    /// Alternate phrases a contact might use for the same item.
    pub aliases: Vec<String>,
    pub sensitivity: Sensitivity,
    pub value: String,
    pub disclosure: DisclosureRule,
}

impl Secret {
    /// Label plus every alias, for referent matching.
    pub fn phrases(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.label.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
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
