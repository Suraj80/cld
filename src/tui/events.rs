#[derive(Debug, Clone)]
pub enum ChatEvent {
    IncomingMessage { from: String, content: String },

    MessageDelivered { peer: String },

    MessageFailed { peer: String, reason: String },

    SystemMessage(String),
}
