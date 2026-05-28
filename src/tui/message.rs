#[derive(Clone, Debug)]
pub enum MessageStatus {
    Sending,
    Delivered,
    Failed,
}

#[derive(Clone, Debug)]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub from: String,
    pub content: String,
    pub timestamp: i64,
    pub direction: MessageDirection,
    pub status: Option<MessageStatus>,
}
