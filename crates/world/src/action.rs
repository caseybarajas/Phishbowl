use crate::ids::SecretId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Principle {
    Reciprocity,
    Scarcity,
    Authority,
    Commitment,
    Liking,
    SocialProof,
    Unity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    Password,
    DoorCode,
    Approval,
    File,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub kind: SecretKind,
    /// What the contact asked for in their own words (e.g. "VPN enrollment code").
    /// The engine matches this against secret labels/aliases to resolve a target.
    pub referent: Option<String>,
    pub target: Option<SecretId>,
    /// Analyst's sensitivity estimate (0..=100) for asks not tied to a known secret.
    pub sensitivity_hint: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coherence {
    InWorld,
    Anomalous,
}

/// What the Analyst extracted from a player message. Evidence, not authority:
/// the referee applies its own rules on top of this parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAction {
    pub principles: Vec<Principle>,
    pub claims: Vec<Claim>,
    /// Commitments and topic mentions the Analyst extracted (distinct from self-claims).
    pub salient_facts: Vec<Claim>,
    pub authority_claim: Option<String>,
    /// A reference the contact offers to back their identity (e.g. a ticket number).
    /// The referee checks it against what the org can actually verify.
    pub verification: Option<String>,
    pub ask: Option<Ask>,
    pub coherence: Coherence,
}

impl ParsedAction {
    /// Conservative fallback used when the Analyst output is missing or malformed:
    /// no principle credit, no claims, no ask. The turn never blocks on a bad parse.
    pub fn inert() -> Self {
        Self {
            principles: Vec::new(),
            claims: Vec::new(),
            salient_facts: Vec::new(),
            authority_claim: None,
            verification: None,
            ask: None,
            coherence: Coherence::InWorld,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Grant,
    Partial,
    Refuse,
    Deflect,
    Stall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    Inconsistency,
    PolicyViolation,
    AuthorityMismatch,
    AuthorityVerified,
    EscalationSpeed,
    ChannelOddity,
    OverPressure,
    FourthWall,
    Rapport,
    PrincipleFit,
    Disclosure,
}

#[derive(Debug, Clone)]
pub struct CausalEntry {
    pub rule: Rule,
    pub weight: i16,
    pub cause: String,
}

#[derive(Debug, Clone)]
pub struct Appraisal {
    pub suspicion_delta: i16,
    pub trust_delta: i16,
    pub reasons: Vec<CausalEntry>,
}

#[derive(Debug, Clone)]
pub struct Adjudication {
    pub verdict: Verdict,
    pub reasons: Vec<CausalEntry>,
}
