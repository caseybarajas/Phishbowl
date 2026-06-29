use crate::action::CausalEntry;
use crate::ids::PersonaId;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub turn: u32,
    pub persona: PersonaId,
    pub entry: CausalEntry,
}

#[derive(Debug, Clone, Default)]
pub struct CausalLog {
    pub entries: Vec<LogEntry>,
}

impl CausalLog {
    pub fn record(&mut self, turn: u32, persona: &PersonaId, reasons: Vec<CausalEntry>) {
        self.entries
            .extend(reasons.into_iter().map(|entry| LogEntry {
                turn,
                persona: persona.clone(),
                entry,
            }));
    }
}
