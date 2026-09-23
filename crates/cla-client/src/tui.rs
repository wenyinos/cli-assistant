//! Full-screen TUI interactive chat mode.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use tokio::sync::mpsc;

use cla_dbus::structures::{Question, StdinInput};

use crate::dbus_client::DbusClient;
use crate::history_payload;

#[derive(Clone)]
enum Role {
    User,
    Assistant,
    Error,
}

struct ChatMessage {
    role: Role,
    text: String,
}

struct TuiState {
    messages: Vec<ChatMessage>,
    input: String,
    pending: bool,
}

enum TuiEvent {
    Response(Result<String, String>),
}

/// Run the TUI until the user quits with Ctrl+C or Ctrl+D.
pub async fn run(
    dbus: DbusClient,
    user_id: String,
    chat_id: String,
    chat_name: String,
    stdin: Option<String>,
    _plain: bool,
) -> i32 {
    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {}", e);
        return 1;
    }

    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        eprintln!("Failed to enter alternate screen: {}", e);
        return 1;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(e) => {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            eprintln!("Failed to create terminal: {}", e);
            return 1;
        }
    };

    let result = run_loop(&mut terminal, dbus, user_id, chat_id, chat_name, stdin).await;

    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    dbus: DbusClient,
    user_id: String,
    chat_id: String,
    chat_name: String,
    stdin: Option<String>,
) -> i32 {
    let (tx, mut rx) = mpsc::unbounded_channel::<TuiEvent>();
    let mut state = TuiState {
        messages: Vec::new(),
        input: String::new(),
        pending: false,
    };

    loop {
        if terminal.draw(|frame| draw(frame, &state)).is_err() {
            return 1;
        }

        if event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Enter if !state.pending && !state.input.trim().is_empty() => {
                        let question_text = state.input.trim().to_string();
                        state.messages.push(ChatMessage {
                            role: Role::User,
                            text: question_text.clone(),
                        });
                        state.input.clear();
                        state.pending = true;

                        let dbus = dbus.clone();
                        let user_id = user_id.clone();
                        let chat_id = chat_id.clone();
                        let chat_name = chat_name.clone();
                        let stdin = stdin.clone();
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            let question = Question {
                                message: question_text,
                                stdin: stdin.map(|s| StdinInput { stdin: s }),
                                attachment: None,
                                terminal: None,
                                systeminfo: None,
                                context_chat: Some(chat_name),
                            };

                            let response = match dbus.ask_question(&user_id, &question).await {
                                Ok(response) => response,
                                Err(e) => {
                                    let _ = tx.send(TuiEvent::Response(Err(e.to_string())));
                                    return;
                                }
                            };

                            let (stored_question, stored_response) =
                                history_payload(&question, &response);
                            let _ = dbus
                                .write_history(
                                    &chat_id,
                                    &user_id,
                                    &stored_question,
                                    &stored_response,
                                )
                                .await;
                            let _ = tx.send(TuiEvent::Response(Ok(response.message)));
                        });
                    }
                    KeyCode::Backspace => {
                        state.input.pop();
                    }
                    KeyCode::Esc => {
                        state.input.clear();
                    }
                    KeyCode::Char(c) => {
                        state.input.push(c);
                    }
                    _ => {}
                }
            }
        }

        while let Ok(event) = rx.try_recv() {
            match event {
                TuiEvent::Response(Ok(text)) => {
                    state.messages.push(ChatMessage {
                        role: Role::Assistant,
                        text,
                    });
                }
                TuiEvent::Response(Err(error)) => {
                    state.messages.push(ChatMessage {
                        role: Role::Error,
                        text: error,
                    });
                }
            }
            state.pending = false;
        }
    }

    0
}

fn draw(frame: &mut Frame, state: &TuiState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(frame.area());

    frame.render_widget(
        Paragraph::new("cli-assistant interactive chat")
            .style(Style::default().add_modifier(Modifier::BOLD)),
        chunks[0],
    );

    let mut items = state
        .messages
        .iter()
        .map(|message| {
            let (prefix, style) = match message.role {
                Role::User => ("You", Style::default().fg(Color::Cyan)),
                Role::Assistant => ("Assistant", Style::default().fg(Color::Green)),
                Role::Error => ("Error", Style::default().fg(Color::Red)),
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}: ", prefix), style),
                Span::raw(message.text.clone()),
            ]))
        })
        .collect::<Vec<_>>();

    if state.pending {
        items.push(ListItem::new(Line::from(Span::styled(
            "Asking...",
            Style::default().fg(Color::Yellow),
        ))));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Conversation "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    let mut list_state = ListState::default();
    if !state.messages.is_empty() {
        list_state.select(Some(state.messages.len() - 1));
    }
    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    frame.render_widget(
        Paragraph::new(state.input.as_str())
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .wrap(Wrap { trim: true }),
        chunks[2],
    );

    frame.render_widget(
        Paragraph::new("Enter send, Esc clear, Ctrl+C/Ctrl+D quit"),
        chunks[3],
    );
}
