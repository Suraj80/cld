use crate::tui::events::ChatEvent;
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
use std::{io, time::Duration};
use tokio::sync::mpsc;
use uuid::Uuid;

pub async fn run_tui() -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn load_chat_history(app: &mut AppState) {
    let Some(peer) = app.contacts.get(app.selected_contact) else {
        return;
    };

    if let Ok(conn) = crate::db::connect() {
        if let Ok(history) = crate::db::get_messages_for_peer(&conn, peer) {
            app.messages = history;
            app.current_peer = Some(peer.clone());
        }
    }
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let config = config::load_or_create_config()?;
    let username = config.username.clone();
    let listen_port = config.listen_port;
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        if let Err(error) = net::listener::listen(listen_port, tx).await {
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
        messages: vec![
            "Welcome to CLD".to_string(),
            "Type a message below".to_string(),
        ],
        input: String::new(),
        selected_contact: 0,
        current_peer: None,
    };

    load_chat_history(&mut app);

    loop {
        while let Ok(event) = rx.try_recv() {
            match event {
                ChatEvent::IncomingMessage { from, content } => {
                    app.messages.push(format!("{from}: {content}"));
                }

                ChatEvent::SystemMessage(message) => {
                    app.messages.push(format!("[system] {message}"));
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

            let message_text = app.messages.join("\n");

            let messages_widget = Paragraph::new(message_text)
                .block(Block::default().title("Messages").borders(Borders::ALL));

            let input_widget = Paragraph::new(app.input.as_str())
                .block(Block::default().title("Input").borders(Borders::ALL));

            let status_widget = Paragraph::new("q: quit | Enter: clear input");

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

                    KeyCode::Down | KeyCode::Char('j') => {
                        if !app.contacts.is_empty() {
                            app.selected_contact = (app.selected_contact + 1) % app.contacts.len();
                            load_chat_history(&mut app);
                        }
                    }

                    KeyCode::Up | KeyCode::Char('k') => {
                        if !app.contacts.is_empty() {
                            if app.selected_contact == 0 {
                                app.selected_contact = app.contacts.len() - 1;
                                load_chat_history(&mut app);
                            } else {
                                app.selected_contact -= 1;
                            }
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
                                .map(String::as_str)
                                .unwrap_or("127.0.0.1:7799");

                            let peer_name = app
                                .contacts
                                .get(app.selected_contact)
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string());

                            app.messages.push(format!("you → {peer_name}: {text}"));

                            match net::sender::send(address, &username, &text).await {
                                Ok(_) => {
                                    app.messages.push("status: delivered".to_string());

                                    if let Ok(conn) = crate::db::connect() {
                                        let _ = crate::db::insert_message(
                                            &conn,
                                            Uuid::new_v4(),
                                            &peer_name,
                                            "out",
                                            &text,
                                            Utc::now().timestamp(),
                                        );
                                    }
                                    load_chat_history(&mut app);
                                }
                                Err(error) => {
                                    app.messages.push(format!("error: {error}"));
                                    app.messages.push(format!("failed → {peer_name}: {text}"));
                                }
                            }
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
