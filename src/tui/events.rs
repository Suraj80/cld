#[derive(Debug, Clone)]
pub enum ChatEvent {
    IncomingMessage {
        from: String,
        content: String,
    },

    MessageDelivered {
        peer: String,
        seq: u64,
    },

    MessageFailed {
        peer: String,
        seq: u64,
        reason: String,
    },

    SystemMessage(String),
}
