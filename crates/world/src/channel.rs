use crate::ids::{ChannelId, PersonaId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Messenger,
    Email,
    Social,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: ChannelId,
    pub kind: ChannelKind,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub turn: u32,
    pub sender: Sender,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sender {
    Player,
    Persona(PersonaId),
}
