use crate::tui::events::ChatEvent;
use crate::tui::message::{ChatMessage, MessageDirection, MessageStatus};
use crate::tui::state::AppState;
use crate::{config, net};
use anyhow::Result;
use chrono::Utc;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::style::{Modifier, Style};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::sync::mpsc;
use uuid::Uuid;

pub async fn run_tui(config: config::Config) -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn load_chat_history(app: &mut AppState) {
    let Some(peer) = app.contacts.get(app.selected_contact) else {
        return;
    };
    let peer = peer.clone();

    if app.messages.contains_key(&peer) {
        app.select_peer(peer);
        return;
    }

    let conn = match crate::db::connect() {
        Ok(conn) => conn,
        Err(_) => {
            app.select_peer(peer);
            return;
        }
    };

    if let Ok(history) = crate::db::get_messages_for_peer(&conn, &peer) {
        let messages = history
            .into_iter()
            .map(|message| {
                let direction = if message.direction == "out" {
                    MessageDirection::Outgoing
                } else {
                    MessageDirection::Incoming
                };

                let from = if message.direction == "out" {
                    "you".to_string()
                } else {
                    peer.clone()
                };

                ChatMessage {
                    from,
                    content: message.content,
                    timestamp: message.timestamp,
                    direction,
                    status: None,
                }
            })
            .collect();
        app.replace_messages_for_peer(peer, messages);
    } else {
        app.select_peer(peer);
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: config::Config,
) -> Result<()> {
    let username = config.username.clone();
    let listen_port = config.listen_port;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sequence_counter = Arc::new(AtomicU64::new(0));
    let listener_username = username.clone();
    let listener_tx = tx.clone();

    tokio::spawn(async move {
        if let Err(error) = net::listener::listen(listen_port, listener_username, listener_tx).await
        {
            eprintln!("Listener failed: {error}");
        }
    });

    let mut app = AppState {
        contacts: config.peers.iter().map(|peer| peer.name.clone()).collect(),
        peer_addresses: config
            .peers
            .iter()
            .map(|peer| peer.address.clone())
            .collect(),
        messages: Default::default(),
        input: String::new(),
        selected_contact: 0,
        current_peer: None,
    };

    load_chat_history(&mut app);

    loop {
        while let Ok(event) = rx.try_recv() {
            match event {
                ChatEvent::IncomingMessage { from, content } => {
                    app.push_message_for_peer(
                        from.clone(),
                        ChatMessage {
                            from,
                            content,
                            timestamp: Utc::now().timestamp(),
                            direction: MessageDirection::Incoming,
                            status: None,
                        },
                    );
                }

                ChatEvent::SystemMessage(message) => {
                    if message == "delivered" {
                        if let Some(last) = app
                            .visible_messages_mut()
                            .and_then(|messages| messages.last_mut())
                        {
                            last.status = Some(MessageStatus::Delivered);
                        }
                    } else if message.starts_with("send failed:") {
                        if let Some(last) = app
                            .visible_messages_mut()
                            .and_then(|messages| messages.last_mut())
                        {
                            last.status = Some(MessageStatus::Failed);
                        }
                    } else {
                        app.push_message_for_current_peer(ChatMessage {
                            from: "system".to_string(),
                            content: message,
                            timestamp: Utc::now().timestamp(),
                            direction: MessageDirection::Incoming,
                            status: None,
                        });
                    }
                }
            }
        }
        terminal.draw(|frame| {
            let area = frame.area();

            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(area);

            let contact_items: Vec<ListItem> = app
                .contacts
                .iter()
                .enumerate()
                .map(|(index, contact)| {
                    if index == app.selected_contact {
                        ListItem::new(format!("> {contact}"))
                            .style(Style::default().add_modifier(Modifier::BOLD))
                    } else {
                        ListItem::new(format!("  {contact}"))
                    }
                })
                .collect();

            let main = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(24), Constraint::Min(1)])
                .split(vertical[0]);

            let contacts_widget = List::new(contact_items)
                .block(Block::default().title("Contacts").borders(Borders::ALL));

            let message_text = app
                .visible_messages()
                .iter()
                .map(|message| {
                    let prefix = match message.direction {
                        MessageDirection::Incoming => format!("{}:", message.from),
                        MessageDirection::Outgoing => "you:".to_string(),
                    };

                    let status = match &message.status {
                        Some(MessageStatus::Sending) => " [sending]",
                        Some(MessageStatus::Delivered) => " [delivered]",
                        Some(MessageStatus::Failed) => " [failed]",
                        None => "",
                    };

                    let time = chrono::DateTime::from_timestamp(message.timestamp, 0)
                        .map(|dt| dt.format("%H:%M").to_string())
                        .unwrap_or_else(|| "--:--".to_string());

                    format!("[{time}] {prefix} {}{}", message.content, status)
                })
                .collect::<Vec<_>>()
                .join("\n");

            let messages_widget = Paragraph::new(message_text)
                .block(Block::default().title("Messages").borders(Borders::ALL));

            let input_widget = Paragraph::new(app.input.as_str())
                .block(Block::default().title("Input").borders(Borders::ALL));

            let status_widget = Paragraph::new("q: quit | ↑/↓: contacts | Enter: send");

            frame.render_widget(contacts_widget, main[0]);
            frame.render_widget(messages_widget, main[1]);
            frame.render_widget(input_widget, vertical[1]);
            frame.render_widget(status_widget, vertical[2]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => break,

                    KeyCode::Down => {
                        if !app.contacts.is_empty() {
                            app.selected_contact = (app.selected_contact + 1) % app.contacts.len();
                            load_chat_history(&mut app);
                        }
                    }

                    KeyCode::Up => {
                        if !app.contacts.is_empty() {
                            if app.selected_contact == 0 {
                                app.selected_contact = app.contacts.len() - 1;
                            } else {
                                app.selected_contact -= 1;
                            }
                            load_chat_history(&mut app);
                        }
                    }

                    KeyCode::Backspace => {
                        app.input.pop();
                    }

                    KeyCode::Enter => {
                        let text = app.input.trim().to_string();

                        if !text.is_empty() {
                            app.input.clear();

                            let address = app
                                .peer_addresses
                                .get(app.selected_contact)
                                .cloned()
                                .unwrap_or_else(|| "127.0.0.1:7799".to_string());

                            let peer_name = app
                                .contacts
                                .get(app.selected_contact)
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string());
                            let expected_fingerprint = config
                                .peers
                                .get(app.selected_contact)
                                .and_then(|peer| peer.expected_fingerprint.as_deref())
                                .map(str::to_owned);

                            app.push_message_for_current_peer(ChatMessage {
                                from: username.clone(),
                                content: text.clone(),
                                timestamp: Utc::now().timestamp(),
                                direction: MessageDirection::Outgoing,
                                status: Some(MessageStatus::Sending),
                            });

                            let tx_clone = tx.clone();
                            let username_clone = username.clone();
                            let peer_name_clone = peer_name.clone();
                            let expected_fingerprint_clone = expected_fingerprint.clone();
                            let text_clone = text.clone();
                            let seq = sequence_counter.fetch_add(1, Ordering::SeqCst);

                            tokio::spawn(async move {
                                let result = net::sender::send(
                                    &address,
                                    &username_clone,
                                    &peer_name_clone,
                                    expected_fingerprint_clone.as_deref(),
                                    &text_clone,
                                    seq,
                                )
                                .await;

                                let message = match result {
                                    Ok(_) => {
                                        if let Ok(conn) = crate::db::connect() {
                                            let _ = crate::db::insert_message(
                                                &conn,
                                                Uuid::new_v4(),
                                                &peer_name_clone,
                                                "out",
                                                &text_clone,
                                                Utc::now().timestamp(),
                                            );
                                        }

                                        ChatEvent::SystemMessage("delivered".to_string())
                                    }
                                    Err(error) => {
                                        ChatEvent::SystemMessage(format!("send failed: {error}"))
                                    }
                                };

                                let _ = tx_clone.send(message);
                            });
                        }
                    }

                    KeyCode::Char(c) => app.input.push(c),

                    _ => {}
                }
            }
        }
    }

    Ok(())
}
