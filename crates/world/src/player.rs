use crate::action::Claim;
use crate::ids::SecretId;

#[derive(Debug, Clone, Default)]
pub struct Player {
    pub pretext: Option<Pretext>,
    pub intel: Vec<Claim>,
    pub disclosed: Vec<SecretId>,
}

#[derive(Debug, Clone)]
pub struct Pretext {
    /// Matched against a secret's `required_pretext`.
    pub label: String,
    pub claimed_identity: String,
    /// Claims to be staff inside the target org — an external contact doing so is a tell.
    pub internal_claim: bool,
    /// Whether the cover can survive an out-of-band check (a callback, a third party).
    pub verifiable: bool,
}
