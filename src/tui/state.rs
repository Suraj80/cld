use crate::tui::message::ChatMessage;

#[derive(Default)]
pub struct AppState {
    pub contacts: Vec<String>,
    pub peer_addresses: Vec<String>,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub selected_contact: usize,
    pub current_peer: Option<String>,
}
