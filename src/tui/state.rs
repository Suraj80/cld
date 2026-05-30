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
    pub fn push_message_for_peer(&mut self, peer: String, message: ChatMessage) {
        self.messages.entry(peer).or_default().push(message);
    }

    pub fn visible_messages(&self) -> &[ChatMessage] {
        self.current_peer
            .as_ref()
            .and_then(|peer| self.messages.get(peer))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::message::{ChatMessage, MessageDirection, MessageStatus};

    #[test]
    fn test_status_update_routes_to_correct_peer() {
        let mut app = AppState {
            contacts: vec!["Alice".to_string(), "Bob".to_string()],
            peer_addresses: vec!["addr_a".to_string(), "addr_b".to_string()],
            messages: HashMap::new(),
            input: String::new(),
            selected_contact: 0,
            current_peer: None,
        };

        // Send a message to peer A (Alice)
        app.push_message_for_peer(
            "Alice".to_string(),
            ChatMessage {
                from: "you".to_string(),
                content: "Hello Alice".to_string(),
                timestamp: 1000,
                direction: MessageDirection::Outgoing,
                status: Some(MessageStatus::Sending),
            },
        );

        // Switch to peer B (Bob)
        app.current_peer = Some("Bob".to_string());

        // Simulate MessageDelivered event for Alice
        if let Some(msgs) = app.messages.get_mut("Alice") {
            if let Some(last) = msgs
                .iter_mut()
                .rev()
                .find(|m| matches!(m.status, Some(MessageStatus::Sending)))
            {
                last.status = Some(MessageStatus::Delivered);
            }
        }

        // Assert: Alice's message now has Delivered status
        let alice_msgs = app.messages.get("Alice").unwrap();
        assert_eq!(alice_msgs.len(), 1);
        assert_eq!(alice_msgs[0].status, Some(MessageStatus::Delivered));
        assert_eq!(alice_msgs[0].content, "Hello Alice");

        // Assert: Bob's buffer is untouched (no messages pushed for Bob)
        let bob_msgs = app.messages.get("Bob");
        assert!(bob_msgs.map_or(true, |msgs| msgs.is_empty()));
    }
}
