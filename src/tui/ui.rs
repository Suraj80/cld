use crate::tui::state::AppState;
use crate::{config, net};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{io, time::Duration};

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

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = AppState {
        contacts: vec!["suraj".to_string(), "friend".to_string()],
        messages: vec![
            "Welcome to CLD".to_string(),
            "Type a message below".to_string(),
        ],
        input: String::new(),
        selected_contact: 0,
    };

    loop {
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

            let main = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(24), Constraint::Min(1)])
                .split(vertical[0]);

            let contact_items: Vec<ListItem> = app
                .contacts
                .iter()
                .map(|c| ListItem::new(c.as_str()))
                .collect();

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
                    KeyCode::Char(c) => app.input.push(c),
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Enter => {
                        let text = app.input.trim().to_string();

                        if !text.is_empty() {
                            app.messages.push(format!("you: {text}"));
                            app.input.clear();

                            let config = config::load_or_create_config()?;
                            let address = "127.0.0.1:7799";

                            match net::sender::send(address, &config.username, &text).await {
                                Ok(_) => app.messages.push("status: sent".to_string()),
                                Err(error) => app.messages.push(format!("error: {error}")),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
