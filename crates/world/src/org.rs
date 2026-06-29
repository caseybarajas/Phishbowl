use crate::action::SecretKind;
use crate::ids::PolicyId;
use crate::persona::Formality;

#[derive(Debug, Clone)]
pub struct Organization {
    pub name: String,
    pub industry: String,
    pub culture: Culture,
    pub policies: Vec<Policy>,
    /// References the org can actually confirm (ticket numbers, callback codes). A
    /// contact citing one of these closes the verification loop the NPC keeps asking for.
    pub verifiable_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Culture {
    pub formality: Formality,
    pub security_conscious: bool,
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub id: PolicyId,
    pub description: String,
    pub forbids_disclosure_of: SecretKind,
}
