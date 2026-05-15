use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;

fn salak_url() -> String {
    std::env::var("SALAK_URL").unwrap_or_else(|_| "http://localhost:8000".into())
}

fn mangosteen_url() -> String {
    std::env::var("MANGOSTEEN_URL").unwrap_or_else(|_| "http://localhost:4000".into())
}

#[derive(Debug, Clone)]
struct App {
    // Auth
    token: Option<String>,
    login_email: String,
    login_password: String,
    login_error: String,
    login_focus: LoginFocus,

    // Nav
    tabs: Vec<&'static str>,
    active_tab: usize,

    // Stock In form
    si_sku: String,
    si_qty: String,
    si_wh: String,
    si_ref: String,
    si_status: String,
    si_focus: FormFocus,

    // Stock Out form
    so_sku: String,
    so_qty: String,
    so_wh: String,
    so_ref: String,
    so_status: String,
    so_focus: FormFocus,

    // Check inventory
    ck_sku: String,
    ck_result: Option<InventoryResult>,

    // Focus on CRUD screen
    input_mode: InputMode,
}

#[derive(Debug, Clone, PartialEq)]
enum LoginFocus {
    Email,
    Password,
    Button,
}

#[derive(Debug, Clone, PartialEq)]
enum FormFocus {
    Sku,
    Qty,
    Warehouse,
    Reference,
    Button,
}

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone)]
struct InventoryResult {
    product_id: i32,
    sku: String,
    name: String,
    real_qty: i32,
}

impl App {
    fn new() -> Self {
        Self {
            token: None,
            login_email: String::new(),
            login_password: String::new(),
            login_error: String::new(),
            login_focus: LoginFocus::Email,
            tabs: vec!["Stock In", "Stock Out", "Check"],
            active_tab: 0,
            si_sku: String::new(),
            si_qty: String::new(),
            si_wh: String::new(),
            si_ref: String::new(),
            si_status: String::new(),
            si_focus: FormFocus::Sku,
            so_sku: String::new(),
            so_qty: String::new(),
            so_wh: String::new(),
            so_ref: String::new(),
            so_status: String::new(),
            so_focus: FormFocus::Sku,
            ck_sku: String::new(),
            ck_result: None,
            input_mode: InputMode::Normal,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load .env if present (ignore error if not found)
    let _ = dotenvy::dotenv();

    let app = App::new();
    let res = run_app(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("{e:?}");
    }
    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> anyhow::Result<()>
where
    <B as Backend>::Error: Send + Sync + 'static,
{
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if app.token.is_none() {
                    handle_login_key(&mut app, key.code);
                } else {
                    handle_main_key(&mut app, key.code).await?;
                }
            }
        }
    }
}

fn handle_login_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Tab => {
            app.login_focus = match app.login_focus {
                LoginFocus::Email => LoginFocus::Password,
                LoginFocus::Password => LoginFocus::Button,
                LoginFocus::Button => LoginFocus::Email,
            };
        }
        KeyCode::BackTab => {
            app.login_focus = match app.login_focus {
                LoginFocus::Email => LoginFocus::Button,
                LoginFocus::Password => LoginFocus::Email,
                LoginFocus::Button => LoginFocus::Password,
            };
        }
        KeyCode::Char(c) => match app.login_focus {
            LoginFocus::Email => app.login_email.push(c),
            LoginFocus::Password => app.login_password.push(c),
            _ => {}
        },
        KeyCode::Backspace => match app.login_focus {
            LoginFocus::Email => {
                app.login_email.pop();
            }
            LoginFocus::Password => {
                app.login_password.pop();
            }
            _ => {}
        },
        KeyCode::Enter => {
            if matches!(app.login_focus, LoginFocus::Button)
                && !app.login_email.is_empty()
                && !app.login_password.is_empty()
            {
                app.login_error.clear();
                let email = app.login_email.clone();
                let password = app.login_password.clone();
                tokio::spawn(async move {
                    match login(&email, &password).await {
                        Ok(token) => {
                            // TODO: set token back to app via channel
                            eprintln!("Login OK: {token}");
                        }
                        Err(e) => {
                            eprintln!("Login ERR: {e}");
                        }
                    }
                });
            }
        }
        KeyCode::Esc => {
            app.login_email.clear();
            app.login_password.clear();
        }
        _ => {}
    }
}

async fn login(_email: &str, _password: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/api/auth/login", mangosteen_url()))
        .json(&serde_json::json!({
            "email": _email,
            "password": _password,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        body["access_token"]
            .as_str()
            .map(String::from)
            .ok_or("No token".into())
    } else {
        Err(format!("Login failed: {}", res.status()))
    }
}

async fn handle_main_key(app: &mut App, key: KeyCode) -> anyhow::Result<()> {
    match key {
        KeyCode::Char('1') => app.active_tab = 0,
        KeyCode::Char('2') => app.active_tab = 1,
        KeyCode::Char('3') => app.active_tab = 2,
        KeyCode::Tab => {
            if app.input_mode == InputMode::Normal {
                app.input_mode = InputMode::Editing;
            }
            let focus = match app.active_tab {
                0 => &mut app.si_focus,
                1 => &mut app.so_focus,
                _ => return Ok(()),
            };
            *focus = match *focus {
                FormFocus::Sku => FormFocus::Qty,
                FormFocus::Qty => FormFocus::Warehouse,
                FormFocus::Warehouse => FormFocus::Reference,
                FormFocus::Reference => FormFocus::Button,
                FormFocus::Button => FormFocus::Sku,
            };
        }
        KeyCode::BackTab => {
            let focus = match app.active_tab {
                0 => &mut app.si_focus,
                1 => &mut app.so_focus,
                _ => return Ok(()),
            };
            *focus = match *focus {
                FormFocus::Sku => FormFocus::Button,
                FormFocus::Qty => FormFocus::Sku,
                FormFocus::Warehouse => FormFocus::Qty,
                FormFocus::Reference => FormFocus::Warehouse,
                FormFocus::Button => FormFocus::Reference,
            };
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => match app.active_tab {
            0 => {
                if matches!(app.si_focus, FormFocus::Button) {
                    stock_in(app).await?;
                }
            }
            1 => {
                if matches!(app.so_focus, FormFocus::Button) {
                    stock_out(app).await?;
                }
            }
            2 => {
                check_inventory(app).await?;
            }
            _ => {}
        },
        KeyCode::Char(c) => {
            if app.input_mode == InputMode::Normal {
                app.input_mode = InputMode::Editing;
            }
            match app.active_tab {
                0 => push_field(
                    &mut app.si_sku,
                    &mut app.si_qty,
                    &mut app.si_wh,
                    &mut app.si_ref,
                    &app.si_focus,
                    c,
                ),
                1 => push_field(
                    &mut app.so_sku,
                    &mut app.so_qty,
                    &mut app.so_wh,
                    &mut app.so_ref,
                    &app.so_focus,
                    c,
                ),
                2 => app.ck_sku.push(c),
                _ => {}
            }
        }
        KeyCode::Backspace => match app.active_tab {
            0 => pop_field(
                &mut app.si_sku,
                &mut app.si_qty,
                &mut app.si_wh,
                &mut app.si_ref,
                &app.si_focus,
            ),
            1 => pop_field(
                &mut app.so_sku,
                &mut app.so_qty,
                &mut app.so_wh,
                &mut app.so_ref,
                &app.so_focus,
            ),
            2 => {
                app.ck_sku.pop();
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

fn push_field(
    sku: &mut String,
    qty: &mut String,
    wh: &mut String,
    rf: &mut String,
    focus: &FormFocus,
    c: char,
) {
    match focus {
        FormFocus::Sku => sku.push(c),
        FormFocus::Qty => qty.push(c),
        FormFocus::Warehouse => wh.push(c),
        FormFocus::Reference => rf.push(c),
        _ => {}
    }
}

fn pop_field(
    sku: &mut String,
    qty: &mut String,
    wh: &mut String,
    rf: &mut String,
    focus: &FormFocus,
) {
    match focus {
        FormFocus::Sku => {
            sku.pop();
        }
        FormFocus::Qty => {
            qty.pop();
        }
        FormFocus::Warehouse => {
            wh.pop();
        }
        FormFocus::Reference => {
            rf.pop();
        }
        _ => {}
    }
}

async fn stock_in(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let qty: i32 = app.si_qty.parse().unwrap_or(0);

    let res = client
        .post(format!("{}/stock-in", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "sku": app.si_sku,
            "quantity": qty,
            "warehouse_id": app.si_wh.parse::<i32>().unwrap_or(1),
            "reference_id": app.si_ref,
        }))
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            app.si_status = format!("OK: +{qty} {}", app.si_sku);
            app.si_sku.clear();
            app.si_qty.clear();
            app.si_ref.clear();
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.si_status = format!("ERR: {body}");
        }
        Err(e) => app.si_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn stock_out(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let qty: i32 = app.so_qty.parse().unwrap_or(0);

    let res = client
        .post(format!("{}/stock-out", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "sku": app.so_sku,
            "quantity": qty,
            "warehouse_id": app.so_wh.parse::<i32>().unwrap_or(1),
            "reference_id": app.so_ref,
        }))
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            app.so_status = format!("OK: -{qty} {}", app.so_sku);
            app.so_sku.clear();
            app.so_qty.clear();
            app.so_ref.clear();
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.so_status = format!("ERR: {body}");
        }
        Err(e) => app.so_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn check_inventory(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");

    let url = format!("{}/inventory?sku={}", salak_url(), app.ck_sku);
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let items = body.as_array().and_then(|a| a.first());
            app.ck_result = items.map(|i| InventoryResult {
                product_id: i["product_id"].as_i64().unwrap_or(0) as i32,
                sku: i["sku"].as_str().unwrap_or("-").into(),
                name: i["name"].as_str().unwrap_or("-").into(),
                real_qty: i["real_qty"].as_i64().unwrap_or(0) as i32,
            });
        }
        Ok(_) => {
            app.ck_result = Some(InventoryResult {
                product_id: 0,
                sku: "ERR".into(),
                name: "Not found or error".into(),
                real_qty: 0,
            });
        }
        Err(e) => {
            app.ck_result = Some(InventoryResult {
                product_id: 0,
                sku: "ERR".into(),
                name: e.to_string(),
                real_qty: 0,
            });
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    if app.token.is_none() {
        login_ui(f, app);
    } else {
        main_ui(f, app);
    }
}

fn login_ui(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 40, f.area());

    let block = Block::default()
        .title(" Duwet — Warehouse Login ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner);

    // Email
    let email_style = if matches!(app.login_focus, LoginFocus::Email) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let email_block = Block::default()
        .title(" Email ")
        .borders(Borders::ALL)
        .style(email_style);
    f.render_widget(
        Paragraph::new(app.login_email.as_str()).block(email_block),
        chunks[0],
    );

    // Password
    let pw_style = if matches!(app.login_focus, LoginFocus::Password) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let masked: String = app.login_password.chars().map(|_| '•').collect();
    let pw_block = Block::default()
        .title(" Password ")
        .borders(Borders::ALL)
        .style(pw_style);
    f.render_widget(Paragraph::new(masked).block(pw_block), chunks[1]);

    // Error
    if !app.login_error.is_empty() {
        f.render_widget(
            Paragraph::new(app.login_error.as_str()).style(Style::default().fg(Color::Red)),
            chunks[2],
        );
    }

    // Login button
    let btn_style = if matches!(app.login_focus, LoginFocus::Button) {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let btn = Paragraph::new("[ Login ]")
        .block(Block::default().borders(Borders::ALL))
        .style(btn_style)
        .alignment(Alignment::Center);
    f.render_widget(btn, chunks[4]);

    // Help
    let help = Paragraph::new("Tab: next  |  Enter: submit  |  Esc: clear")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);
}

fn main_ui(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    // Tabs
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == app.active_tab {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(format!(" {t} [{i}] ")).style(style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().title(" Duwet ").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Cyan))
        .divider("|");
    f.render_widget(tabs, chunks[0]);

    match app.active_tab {
        0 => stock_in_ui(f, app, chunks[1]),
        1 => stock_out_ui(f, app, chunks[1]),
        2 => check_ui(f, app, chunks[1]),
        _ => {}
    }
}

fn stock_in_ui(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let fields = [
        ("SKU", app.si_sku.as_str(), &app.si_focus, FormFocus::Sku),
        ("Quantity", app.si_qty.as_str(), &app.si_focus, FormFocus::Qty),
        (
            "Warehouse ID",
            app.si_wh.as_str(),
            &app.si_focus,
            FormFocus::Warehouse,
        ),
        (
            "Reference",
            app.si_ref.as_str(),
            &app.si_focus,
            FormFocus::Reference,
        ),
    ];

    for (i, (label, value, focus, expected)) in fields.iter().enumerate() {
        let style = if **focus == *expected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title(format!(" {label} "))
            .borders(Borders::ALL)
            .style(style);
        f.render_widget(Paragraph::new(*value).block(block), chunks[i]);
    }

    // Submit button
    let btn_style = if matches!(app.si_focus, FormFocus::Button) {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let btn = Paragraph::new("[ Stock In ]")
        .block(Block::default().borders(Borders::ALL))
        .style(btn_style)
        .alignment(Alignment::Center);
    f.render_widget(btn, chunks[4]);

    // Status
    let status_style = if app.si_status.starts_with("OK") {
        Style::default().fg(Color::Green)
    } else if app.si_status.starts_with("ERR") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new(app.si_status.as_str()).style(status_style),
        chunks[5],
    );
}

fn stock_out_ui(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let fields = [
        ("SKU", app.so_sku.as_str(), &app.so_focus, FormFocus::Sku),
        ("Quantity", app.so_qty.as_str(), &app.so_focus, FormFocus::Qty),
        (
            "Warehouse ID",
            app.so_wh.as_str(),
            &app.so_focus,
            FormFocus::Warehouse,
        ),
        (
            "Reference",
            app.so_ref.as_str(),
            &app.so_focus,
            FormFocus::Reference,
        ),
    ];

    for (i, (label, value, focus, expected)) in fields.iter().enumerate() {
        let style = if **focus == *expected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title(format!(" {label} "))
            .borders(Borders::ALL)
            .style(style);
        f.render_widget(Paragraph::new(*value).block(block), chunks[i]);
    }

    let btn_style = if matches!(app.so_focus, FormFocus::Button) {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let btn = Paragraph::new("[ Stock Out ]")
        .block(Block::default().borders(Borders::ALL))
        .style(btn_style)
        .alignment(Alignment::Center);
    f.render_widget(btn, chunks[4]);

    let status_style = if app.so_status.starts_with("OK") {
        Style::default().fg(Color::Green)
    } else if app.so_status.starts_with("ERR") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new(app.so_status.as_str()).style(status_style),
        chunks[5],
    );
}

fn check_ui(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let block = Block::default()
        .title(" SKU ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(
        Paragraph::new(app.ck_sku.as_str()).block(block),
        chunks[0],
    );

    if let Some(ref result) = app.ck_result {
        let rows = vec![
            Line::from(format!("  Product ID : {}", result.product_id)),
            Line::from(format!("  SKU        : {}", result.sku)),
            Line::from(format!("  Name       : {}", result.name)),
            Line::from(""),
            Line::from(Span::styled(
                format!("  Stock      : {} pcs", result.real_qty),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        f.render_widget(
            Paragraph::new(Text::from(rows))
                .block(Block::default().title(" Result ").borders(Borders::ALL)),
            chunks[1],
        );
    } else {
        f.render_widget(
            Paragraph::new("Press Enter to check inventory")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            chunks[1],
        );
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
