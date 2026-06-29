use crate::ids::SecretId;

#[derive(Debug, Clone)]
pub struct Objective {
    pub description: String,
    pub success: SuccessCondition,
    pub turn_budget: u32,
}

#[derive(Debug, Clone)]
pub enum SuccessCondition {
    SecretDisclosed(SecretId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Active,
    Won,
    Detected,
    OutOfTurns,
}
