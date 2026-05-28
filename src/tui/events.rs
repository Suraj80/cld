#[derive(Debug, Clone)]
pub enum ChatEvent {
    IncomingMessage { from: String, content: String },

    SystemMessage(String),
}
