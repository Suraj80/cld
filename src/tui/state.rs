use crate::tui::message::ChatMessage;
use std::collections::HashMap;

#[derive(Default)]
pub struct AppState {
    pub contacts: Vec<String>,
    pub peer_addresses: Vec<String>,
    pub messages: HashMap<String, Vec<ChatMessage>>,
    pub input: String,
    pub selected_contact: usize,
    pub current_peer: Option<String>,
}

impl AppState {
    pub fn visible_messages(&self) -> &[ChatMessage] {
        let Some(peer) = self.current_peer.as_deref() else {
            return &[];
        };

        self.messages.get(peer).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn visible_messages_mut(&mut self) -> Option<&mut Vec<ChatMessage>> {
        let peer = self.current_peer.as_deref()?;
        self.messages.get_mut(peer)
    }

    pub fn replace_messages_for_peer(&mut self, peer: String, messages: Vec<ChatMessage>) {
        self.current_peer = Some(peer.clone());
        self.messages.insert(peer, messages);
    }

    pub fn select_peer(&mut self, peer: String) {
        self.current_peer = Some(peer.clone());
        self.messages.entry(peer).or_default();
    }

    pub fn push_message_for_peer(&mut self, peer: String, message: ChatMessage) {
        self.messages.entry(peer).or_default().push(message);
    }

    pub fn push_message_for_current_peer(&mut self, message: ChatMessage) {
        if let Some(peer) = self.current_peer.clone() {
            self.push_message_for_peer(peer, message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::message::{MessageDirection, MessageStatus};

    fn incoming(from: &str, content: &str) -> ChatMessage {
        ChatMessage {
            from: from.to_string(),
            content: content.to_string(),
            timestamp: 1,
            direction: MessageDirection::Incoming,
            status: None,
        }
    }

    #[test]
    fn buffers_messages_for_inactive_peers() {
        let mut app = AppState {
            current_peer: Some("bob".to_string()),
            ..Default::default()
        };

        app.push_message_for_peer("carol".to_string(), incoming("carol", "hello"));

        assert!(app.visible_messages().is_empty());

        app.current_peer = Some("carol".to_string());
        let visible = app.visible_messages();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].from, "carol");
        assert_eq!(visible[0].content, "hello");
    }

    #[test]
    fn status_updates_apply_to_current_peer_only() {
        let mut app = AppState {
            current_peer: Some("bob".to_string()),
            ..Default::default()
        };
        app.push_message_for_peer(
            "bob".to_string(),
            ChatMessage {
                from: "me".to_string(),
                content: "to bob".to_string(),
                timestamp: 1,
                direction: MessageDirection::Outgoing,
                status: Some(MessageStatus::Sending),
            },
        );
        app.push_message_for_peer("carol".to_string(), incoming("carol", "hello"));

        app.visible_messages_mut().unwrap()[0].status = Some(MessageStatus::Delivered);

        assert!(matches!(
            app.messages["bob"][0].status,
            Some(MessageStatus::Delivered)
        ));
        assert!(app.messages["carol"][0].status.is_none());
    }

    #[test]
    fn selecting_peer_preserves_buffered_messages() {
        let mut app = AppState::default();

        app.push_message_for_peer("carol".to_string(), incoming("carol", "hello"));
        app.select_peer("carol".to_string());

        assert_eq!(app.visible_messages().len(), 1);
        assert_eq!(app.visible_messages()[0].content, "hello");
    }
}
