use serde::Deserialize;
use world::{Formality, Mood, SecretKind, Tuning};

#[derive(Debug, Deserialize)]
pub struct ScenarioPackage {
    pub meta: Meta,
    pub organization: OrgDto,
    pub cast: Vec<PersonaDto>,
    pub secrets: Vec<SecretDto>,
    #[serde(default)]
    pub facts: Vec<FactDto>,
    pub objective: ObjectiveDto,
    pub seed: SeedDto,
    #[serde(default)]
    pub tuning: Option<TuningDto>,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    pub id: String,
    pub title: String,
    pub author: String,
    pub premise: String,
    pub difficulty: String,
}

#[derive(Debug, Deserialize)]
pub struct OrgDto {
    pub name: String,
    pub industry: String,
    pub formality: FormalityDto,
    pub security_conscious: bool,
    #[serde(default)]
    pub policies: Vec<PolicyDto>,
    #[serde(default)]
    pub verifiable_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PolicyDto {
    pub id: String,
    pub description: String,
    pub forbids_disclosure_of: SecretKindDto,
}

#[derive(Debug, Deserialize)]
pub struct PersonaDto {
    pub id: String,
    pub name: String,
    pub title: String,
    pub department: String,
    #[serde(default)]
    pub reports_to: Option<String>,
    pub personality: PersonalityDto,
    pub voice: VoiceDto,
    #[serde(default)]
    pub knowledge: Vec<String>,
    #[serde(default)]
    pub facts: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<RelationshipDto>,
    #[serde(default)]
    pub hooks: Vec<String>,
    #[serde(default)]
    pub red_lines: Vec<SecretKindDto>,
    #[serde(default)]
    pub start: StartStateDto,
}

#[derive(Debug, Deserialize)]
pub struct PersonalityDto {
    pub agreeableness: u8,
    pub conscientiousness: u8,
    pub security_awareness: u8,
    pub busyness: u8,
    pub ego: u8,
    pub helpfulness: u8,
}

#[derive(Debug, Deserialize)]
pub struct VoiceDto {
    pub style: String,
    pub formality: FormalityDto,
    pub avg_words: u16,
    #[serde(default)]
    pub quirks: Vec<String>,
    #[serde(default)]
    pub emoji: bool,
}

#[derive(Debug, Deserialize)]
pub struct RelationshipDto {
    pub other: String,
    pub trust: u8,
    #[serde(default)]
    pub would_warn: bool,
    #[serde(default)]
    pub defers_to: bool,
}

#[derive(Debug, Deserialize)]
pub struct StartStateDto {
    pub suspicion: u8,
    pub trust: u8,
    pub mood: MoodDto,
}

impl Default for StartStateDto {
    fn default() -> Self {
        Self {
            suspicion: 10,
            trust: 10,
            mood: MoodDto::Neutral,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SecretDto {
    pub id: String,
    pub owner: String,
    pub kind: SecretKindDto,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub sensitivity: u8,
    pub value: String,
    pub disclosure: DisclosureDto,
}

#[derive(Debug, Deserialize)]
pub struct DisclosureDto {
    pub trust_min: u8,
    pub suspicion_max: u8,
    #[serde(default)]
    pub required_pretext: Option<String>,
    #[serde(default)]
    pub requires_authorization: bool,
}

#[derive(Debug, Deserialize)]
pub struct FactDto {
    pub id: String,
    pub owner: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct ObjectiveDto {
    pub description: String,
    pub success: SuccessDto,
    pub turn_budget: u32,
}

#[derive(Debug, Deserialize)]
pub enum SuccessDto {
    SecretDisclosed(String),
}

#[derive(Debug, Deserialize)]
pub struct SeedDto {
    #[serde(default)]
    pub pretext: Option<PretextDto>,
    #[serde(default)]
    pub intel: Vec<ClaimDto>,
}

#[derive(Debug, Deserialize)]
pub struct PretextDto {
    pub label: String,
    pub claimed_identity: String,
    #[serde(default)]
    pub internal_claim: bool,
    #[serde(default)]
    pub verifiable: bool,
}

#[derive(Debug, Deserialize)]
pub struct ClaimDto {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum FormalityDto {
    Casual,
    Neutral,
    Formal,
}

impl From<FormalityDto> for Formality {
    fn from(value: FormalityDto) -> Self {
        match value {
            FormalityDto::Casual => Formality::Casual,
            FormalityDto::Neutral => Formality::Neutral,
            FormalityDto::Formal => Formality::Formal,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum MoodDto {
    Friendly,
    Neutral,
    Busy,
    Annoyed,
    Away,
}

impl From<MoodDto> for Mood {
    fn from(value: MoodDto) -> Self {
        match value {
            MoodDto::Friendly => Mood::Friendly,
            MoodDto::Neutral => Mood::Neutral,
            MoodDto::Busy => Mood::Busy,
            MoodDto::Annoyed => Mood::Annoyed,
            MoodDto::Away => Mood::Away,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SecretKindDto {
    Password,
    DoorCode,
    Approval,
    File,
    Info,
}

impl From<SecretKindDto> for SecretKind {
    fn from(value: SecretKindDto) -> Self {
        match value {
            SecretKindDto::Password => SecretKind::Password,
            SecretKindDto::DoorCode => SecretKind::DoorCode,
            SecretKindDto::Approval => SecretKind::Approval,
            SecretKindDto::File => SecretKind::File,
            SecretKindDto::Info => SecretKind::Info,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TuningDto {
    pub w_inconsistency: Option<i16>,
    pub w_policy: Option<i16>,
    pub w_authority: Option<i16>,
    pub w_verification: Option<i16>,
    pub w_channel_oddity: Option<i16>,
    pub w_over_pressure: Option<i16>,
    pub w_fourth_wall: Option<i16>,
    pub w_rapport: Option<i16>,
    pub w_principle_fit: Option<i16>,
    pub escalation_pct: Option<i16>,
    pub over_pressure_threshold: Option<usize>,
    pub partial_trust_band: Option<u8>,
    pub report_threshold: Option<u8>,
    pub detection_threshold: Option<u8>,
    pub report_org_bump: Option<i16>,
}

impl TuningDto {
    pub fn merge(self, base: Tuning) -> Tuning {
        Tuning {
            w_inconsistency: self.w_inconsistency.unwrap_or(base.w_inconsistency),
            w_policy: self.w_policy.unwrap_or(base.w_policy),
            w_authority: self.w_authority.unwrap_or(base.w_authority),
            w_verification: self.w_verification.unwrap_or(base.w_verification),
            w_channel_oddity: self.w_channel_oddity.unwrap_or(base.w_channel_oddity),
            w_over_pressure: self.w_over_pressure.unwrap_or(base.w_over_pressure),
            w_fourth_wall: self.w_fourth_wall.unwrap_or(base.w_fourth_wall),
            w_rapport: self.w_rapport.unwrap_or(base.w_rapport),
            w_principle_fit: self.w_principle_fit.unwrap_or(base.w_principle_fit),
            escalation_pct: self.escalation_pct.unwrap_or(base.escalation_pct),
            over_pressure_threshold: self
                .over_pressure_threshold
                .unwrap_or(base.over_pressure_threshold),
            partial_trust_band: self.partial_trust_band.unwrap_or(base.partial_trust_band),
            report_threshold: self
                .report_threshold
                .map_or(base.report_threshold, world::Suspicion::new),
            detection_threshold: self
                .detection_threshold
                .map_or(base.detection_threshold, world::Suspicion::new),
            report_org_bump: self.report_org_bump.unwrap_or(base.report_org_bump),
        }
    }
}
