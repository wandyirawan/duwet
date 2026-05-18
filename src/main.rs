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
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs},
    Frame, Terminal,
};
use std::io;

fn salak_url() -> String {
    std::env::var("SALAK_URL").unwrap_or_else(|_| "http://localhost:8000".into())
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

    // Transactions
    txn_rows: Vec<TransactionRow>,
    txn_scroll: usize,
    txn_status: String,

    // Categories
    cat_rows: Vec<CategoryRow>,
    cat_scroll: usize,
    cat_status: String,
    cat_mode: CategoryMode,
    cat_name: String,
    cat_selected: Option<i32>,

    // Warehouses
    wh_rows: Vec<WarehouseRow>,
    wh_scroll: usize,
    wh_status: String,
    wh_mode: WhMode,
    wh_name: String,
    wh_location: String,
    wh_focus: WhFormFocus,
    wh_selected: Option<i32>,
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

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct TransactionRow {
    id: i32,
    product_id: i32,
    warehouse_id: i32,
    delta_qty: serde_json::Number,
    reference_id: Option<String>,
    notes: Option<String>,
    created_at: String,
    product_name: Option<String>,
    sku: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct CategoryRow {
    id: i32,
    name: String,
    slug: String,
}

#[derive(Debug, Clone, PartialEq)]
enum CategoryMode {
    Browse,
    Create,
    Edit(i32),
    DeleteConfirm(i32, String),
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct WarehouseRow {
    id: i32,
    name: String,
    location: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum WhFormFocus {
    Name,
    Location,
}

#[derive(Debug, Clone, PartialEq)]
enum WhMode {
    Browse,
    Create,
    Edit(i32),
    DeleteConfirm(i32, String),
}

impl App {
    fn new() -> Self {
        Self {
            token: None,
            login_email: String::new(),
            login_password: String::new(),
            login_error: String::new(),
            login_focus: LoginFocus::Email,
            tabs: vec!["Stock In", "Stock Out", "Check", "Transactions", "Categories", "Warehouses"],
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
            txn_rows: vec![],
            txn_scroll: 0,
            txn_status: String::new(),
            cat_rows: vec![],
            cat_scroll: 0,
            cat_status: String::new(),
            cat_mode: CategoryMode::Browse,
            cat_name: String::new(),
            cat_selected: None,
            wh_rows: vec![],
            wh_scroll: 0,
            wh_status: String::new(),
            wh_mode: WhMode::Browse,
            wh_name: String::new(),
            wh_location: String::new(),
            wh_focus: WhFormFocus::Name,
            wh_selected: None,
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
        .post(format!("{}/auth/login", salak_url()))
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
        KeyCode::Char('4') => app.active_tab = 3,
        KeyCode::Char('5') => {
            app.active_tab = 4;
            if app.cat_rows.is_empty() {
                fetch_categories(app).await?;
            }
        }
        KeyCode::Char('6') => {
            app.active_tab = 5;
            if app.wh_rows.is_empty() {
                fetch_warehouses(app).await?;
            }
        }
        KeyCode::Tab => {
            if app.input_mode == InputMode::Normal {
                app.input_mode = InputMode::Editing;
            }
            if app.active_tab == 5 {
                app.wh_focus = match app.wh_focus {
                    WhFormFocus::Name => WhFormFocus::Location,
                    WhFormFocus::Location => WhFormFocus::Name,
                };
                return Ok(());
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
            if app.active_tab == 5 {
                app.wh_focus = match app.wh_focus {
                    WhFormFocus::Name => WhFormFocus::Location,
                    WhFormFocus::Location => WhFormFocus::Name,
                };
                return Ok(());
            }
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
            if app.active_tab == 4 {
                app.cat_mode = CategoryMode::Browse;
                app.cat_name.clear();
            }
            if app.active_tab == 5 {
                app.wh_mode = WhMode::Browse;
                app.wh_name.clear();
                app.wh_location.clear();
            }
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
            3 => {
                fetch_transactions(app).await?;
            },
            4 => match app.cat_mode.clone() {
                CategoryMode::Browse => {
                    if let Some(id) = app.cat_selected {
                        if let Some(cat) = app.cat_rows.iter().find(|c| c.id == id) {
                            app.cat_name = cat.name.clone();
                            app.cat_mode = CategoryMode::Edit(id);
                        }
                    } else if app.cat_rows.is_empty() {
                        fetch_categories(app).await?;
                    }
                }
                CategoryMode::Create => {
                    create_category(app).await?;
                }
                CategoryMode::Edit(_) => {
                    update_category(app).await?;
                }
                CategoryMode::DeleteConfirm(_id, _name) => {
                    delete_category(app).await?;
                }
            },
            5 => match app.wh_mode.clone() {
                WhMode::Browse => {
                    if let Some(id) = app.wh_selected {
                        if let Some(wh) = app.wh_rows.iter().find(|w| w.id == id) {
                            app.wh_name = wh.name.clone();
                            app.wh_location = wh.location.clone().unwrap_or_default();
                            app.wh_mode = WhMode::Edit(id);
                        }
                    } else if app.wh_rows.is_empty() {
                        fetch_warehouses(app).await?;
                    }
                }
                WhMode::Create => {
                    create_warehouse(app).await?;
                }
                WhMode::Edit(_) => {
                    update_warehouse(app).await?;
                }
                WhMode::DeleteConfirm(_id, _name) => {
                    delete_warehouse(app).await?;
                }
            },
            _ => {}
        },
        KeyCode::Up => {
            if app.active_tab == 3 && app.txn_scroll > 0 {
                app.txn_scroll -= 1;
            }
            if app.active_tab == 4 && app.cat_scroll > 0 {
                app.cat_scroll -= 1;
            }
            if app.active_tab == 5 && app.wh_scroll > 0 {
                app.wh_scroll -= 1;
            }
        }
        KeyCode::Down => {
            if app.active_tab == 3 {
                app.txn_scroll += 1;
            }
            if app.active_tab == 4 {
                app.cat_scroll += 1;
            }
            if app.active_tab == 5 {
                app.wh_scroll += 1;
            }
        }
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
                4 => match app.cat_mode {
                    CategoryMode::Create | CategoryMode::Edit(_) => {
                        app.cat_name.push(c);
                    }
                    _ => {
                        if c == 'n' {
                            app.cat_mode = CategoryMode::Create;
                            app.cat_name.clear();
                        } else if c == 'd' {
                            if let Some(id) = app.cat_selected {
                                if let Some(cat) = app.cat_rows.iter().find(|r| r.id == id) {
                                    app.cat_mode = CategoryMode::DeleteConfirm(id, cat.name.clone());
                                }
                            }
                        }
                    }
                },
                5 => match app.wh_mode {
                    WhMode::Create | WhMode::Edit(_) => {
                        match app.wh_focus {
                            WhFormFocus::Name => app.wh_name.push(c),
                            WhFormFocus::Location => app.wh_location.push(c),
                        }
                    }
                    _ => {
                        if c == 'n' {
                            app.wh_mode = WhMode::Create;
                            app.wh_name.clear();
                            app.wh_location.clear();
                        } else if c == 'd' {
                            if let Some(id) = app.wh_selected {
                                if let Some(wh) = app.wh_rows.iter().find(|r| r.id == id) {
                                    app.wh_mode = WhMode::DeleteConfirm(id, wh.name.clone());
                                }
                            }
                        }
                    }
                },
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
            4 => {
                app.cat_name.pop();
            }
            5 => {
                match app.wh_focus {
                    WhFormFocus::Name => app.wh_name.pop(),
                    WhFormFocus::Location => app.wh_location.pop(),
                };
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

async fn fetch_transactions(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");

    let url = format!("{}/inventory/transactions?limit=50", salak_url());
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            match r.json::<Vec<TransactionRow>>().await {
                Ok(rows) => {
                    app.txn_rows = rows;
                    app.txn_scroll = 0;
                    app.txn_status = format!("{} transactions", app.txn_rows.len());
                }
                Err(e) => app.txn_status = format!("Parse err: {e}"),
            }
        }
        Ok(r) => {
            app.txn_status = format!("HTTP {}", r.status());
        }
        Err(e) => app.txn_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn fetch_categories(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let res = client
        .get(format!("{}/categories", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            match r.json::<Vec<CategoryRow>>().await {
                Ok(rows) => {
                    app.cat_rows = rows;
                    app.cat_scroll = 0;
                    app.cat_status = format!("{} categories", app.cat_rows.len());
                }
                Err(e) => app.cat_status = format!("Parse err: {e}"),
            }
        }
        Ok(r) => app.cat_status = format!("HTTP {}", r.status()),
        Err(e) => app.cat_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn create_category(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let name = app.cat_name.trim().to_string();
    if name.is_empty() {
        app.cat_status = "ERR: Name required".into();
        return Ok(());
    }
    let res = client
        .post(format!("{}/categories", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            app.cat_status = "Created".into();
            app.cat_name.clear();
            app.cat_mode = CategoryMode::Browse;
            fetch_categories(app).await?;
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.cat_status = format!("ERR: {body}");
        }
        Err(e) => app.cat_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn update_category(app: &mut App) -> anyhow::Result<()> {
    let id = match app.cat_mode {
        CategoryMode::Edit(id) => id,
        _ => return Ok(()),
    };
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let name = app.cat_name.trim().to_string();
    if name.is_empty() {
        app.cat_status = "ERR: Name required".into();
        return Ok(());
    }
    let res = client
        .put(format!("{}/categories/{id}", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            app.cat_status = "Updated".into();
            app.cat_name.clear();
            app.cat_mode = CategoryMode::Browse;
            fetch_categories(app).await?;
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.cat_status = format!("ERR: {body}");
        }
        Err(e) => app.cat_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn delete_category(app: &mut App) -> anyhow::Result<()> {
    let id = match app.cat_mode {
        CategoryMode::DeleteConfirm(id, _) => id,
        _ => return Ok(()),
    };
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let res = client
        .delete(format!("{}/categories/{id}", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            app.cat_status = "Deleted".into();
            app.cat_mode = CategoryMode::Browse;
            app.cat_selected = None;
            fetch_categories(app).await?;
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.cat_status = format!("ERR: {body}");
        }
        Err(e) => app.cat_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn fetch_warehouses(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let res = client
        .get(format!("{}/warehouses", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            match r.json::<Vec<WarehouseRow>>().await {
                Ok(rows) => {
                    app.wh_rows = rows;
                    app.wh_scroll = 0;
                    app.wh_status = format!("{} warehouses", app.wh_rows.len());
                }
                Err(e) => app.wh_status = format!("Parse err: {e}"),
            }
        }
        Ok(r) => app.wh_status = format!("HTTP {}", r.status()),
        Err(e) => app.wh_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn create_warehouse(app: &mut App) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let name = app.wh_name.trim().to_string();
    if name.is_empty() {
        app.wh_status = "ERR: Name required".into();
        return Ok(());
    }
    let res = client
        .post(format!("{}/warehouses", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": name,
            "location": app.wh_location.trim(),
        }))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            app.wh_status = "Created".into();
            app.wh_name.clear();
            app.wh_location.clear();
            app.wh_mode = WhMode::Browse;
            fetch_warehouses(app).await?;
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.wh_status = format!("ERR: {body}");
        }
        Err(e) => app.wh_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn update_warehouse(app: &mut App) -> anyhow::Result<()> {
    let id = match app.wh_mode {
        WhMode::Edit(id) => id,
        _ => return Ok(()),
    };
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let name = app.wh_name.trim().to_string();
    if name.is_empty() {
        app.wh_status = "ERR: Name required".into();
        return Ok(());
    }
    let res = client
        .put(format!("{}/warehouses/{id}", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": name,
            "location": app.wh_location.trim(),
        }))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            app.wh_status = "Updated".into();
            app.wh_name.clear();
            app.wh_location.clear();
            app.wh_mode = WhMode::Browse;
            fetch_warehouses(app).await?;
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.wh_status = format!("ERR: {body}");
        }
        Err(e) => app.wh_status = format!("ERR: {e}"),
    }
    Ok(())
}

async fn delete_warehouse(app: &mut App) -> anyhow::Result<()> {
    let id = match app.wh_mode {
        WhMode::DeleteConfirm(id, _) => id,
        _ => return Ok(()),
    };
    let client = reqwest::Client::new();
    let token = app.token.as_deref().unwrap_or("");
    let res = client
        .delete(format!("{}/warehouses/{id}", salak_url()))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => {
            app.wh_status = "Deleted".into();
            app.wh_mode = WhMode::Browse;
            app.wh_selected = None;
            fetch_warehouses(app).await?;
        }
        Ok(r) => {
            let body = r.text().await.unwrap_or_default();
            app.wh_status = format!("ERR: {body}");
        }
        Err(e) => app.wh_status = format!("ERR: {e}"),
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
        3 => transactions_ui(f, app, chunks[1]),
        4 => categories_ui(f, app, chunks[1]),
        5 => warehouses_ui(f, app, chunks[1]),
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

fn transactions_ui(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Status line
    let status_style = if app.txn_status.starts_with("ERR") {
        Style::default().fg(Color::Red)
    } else if !app.txn_status.is_empty() {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };
    f.render_widget(
        Paragraph::new(app.txn_status.as_str())
            .style(status_style),
        chunks[0],
    );

    if app.txn_rows.is_empty() {
        f.render_widget(
            Paragraph::new("Press Enter to load transactions")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            chunks[1],
        );
        return;
    }

    // Build table rows
    let header_cells = ["Time", "SKU", "Delta", "Ref", "WH"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .txn_rows
        .iter()
        .map(|t| {
            let time = &t.created_at[..19]; // truncate ISO to YYYY-MM-DD HH:MM:SS
            let sku = t.sku.as_deref().unwrap_or("-");
            let delta = &t.delta_qty;
            let ref_id = t.reference_id.as_deref().unwrap_or("-");
            let wh = t.warehouse_id.to_string();
            let delta_style = if delta.as_f64().unwrap_or(0.0) >= 0.0 {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            Row::new(vec![
                Cell::from(time),
                Cell::from(sku),
                Cell::from(delta.to_string()).style(delta_style),
                Cell::from(ref_id),
                Cell::from(wh),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .block(Block::default().title(" Transactions ").borders(Borders::ALL))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, chunks[1], &mut ratatui::widgets::TableState::new().with_offset(app.txn_scroll));
}

fn categories_ui(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Status line
    let status_style = if app.cat_status.starts_with("ERR") {
        Style::default().fg(Color::Red)
    } else if !app.cat_status.is_empty() {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };
    f.render_widget(Paragraph::new(app.cat_status.as_str()).style(status_style), chunks[0]);

    match app.cat_mode {
        CategoryMode::Browse => {
            if app.cat_rows.is_empty() {
                f.render_widget(
                    Paragraph::new("Press Enter or [5] to load categories\n\nn = new  |  Enter = edit  |  d = delete  |  ↑↓ = scroll")
                        .style(Style::default().fg(Color::DarkGray))
                        .alignment(Alignment::Center),
                    chunks[1],
                );
                return;
            }

            let header_cells = ["ID", "Name", "Slug"]
                .iter()
                .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
            let header = Row::new(header_cells).height(1).bottom_margin(1);

            let rows: Vec<Row> = app.cat_rows.iter().map(|c| {
                Row::new(vec![
                    Cell::from(c.id.to_string()),
                    Cell::from(c.name.as_str()),
                    Cell::from(c.slug.as_str()),
                ])
            }).collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Length(6),
                    Constraint::Length(20),
                    Constraint::Length(20),
                ],
            )
            .header(header)
            .block(Block::default().title(" Categories ").borders(Borders::ALL))
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

            f.render_stateful_widget(
                table,
                chunks[1],
                &mut ratatui::widgets::TableState::new().with_offset(app.cat_scroll),
            );
        }
        CategoryMode::Create | CategoryMode::Edit(_) => {
            let is_edit = matches!(app.cat_mode, CategoryMode::Edit(_));

            let form_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(chunks[1]);

            let name_block = Block::default()
                .title(" Name ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(Paragraph::new(app.cat_name.as_str()).block(name_block), form_chunks[0]);

            let btn_style = Style::default()
                .fg(Color::Black)
                .bg(if is_edit { Color::Blue } else { Color::Green })
                .add_modifier(Modifier::BOLD);
            let btn = Paragraph::new(if is_edit { "[ Update ]" } else { "[ Create ]" })
                .block(Block::default().borders(Borders::ALL))
                .style(btn_style)
                .alignment(Alignment::Center);
            f.render_widget(btn, form_chunks[1]);

            f.render_widget(
                Paragraph::new("Enter: submit  |  Esc: back").style(Style::default().fg(Color::DarkGray)).alignment(Alignment::Center),
                form_chunks[2],
            );
        }
        CategoryMode::DeleteConfirm(_, ref name) => {
            let popup = centered_rect(40, 20, chunks[1]);
            f.render_widget(Clear, popup);

            let inner = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(popup);

            let msg = format!(" Delete category \"{name}\"?\n\n This will fail if products still reference it.");
            f.render_widget(
                Paragraph::new(msg.as_str())
                    .block(Block::default().title(" Confirm Delete ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center),
                inner[0],
            );

            f.render_widget(
                Paragraph::new("Enter: confirm  |  Esc: cancel")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                inner[1],
            );
        }
    }
}

fn warehouses_ui(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let status_style = if app.wh_status.starts_with("ERR") {
        Style::default().fg(Color::Red)
    } else if !app.wh_status.is_empty() {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };
    f.render_widget(Paragraph::new(app.wh_status.as_str()).style(status_style), chunks[0]);

    match app.wh_mode {
        WhMode::Browse => {
            if app.wh_rows.is_empty() {
                f.render_widget(
                    Paragraph::new("Press Enter or [6] to load warehouses\n\nn = new  |  Enter = edit  |  d = delete  |  ↑↓ = scroll\nTab = switch field")
                        .style(Style::default().fg(Color::DarkGray))
                        .alignment(Alignment::Center),
                    chunks[1],
                );
                return;
            }

            let header = Row::new(
                ["ID", "Name", "Location"]
                    .iter().map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            ).height(1).bottom_margin(1);

            let rows: Vec<Row> = app.wh_rows.iter().map(|w| {
                Row::new(vec![
                    Cell::from(w.id.to_string()),
                    Cell::from(w.name.as_str()),
                    Cell::from(w.location.as_deref().unwrap_or("-")),
                ])
            }).collect();

            let table = Table::new(rows, [
                Constraint::Length(6),
                Constraint::Length(20),
                Constraint::Length(20),
            ])
            .header(header)
            .block(Block::default().title(" Warehouses ").borders(Borders::ALL))
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

            f.render_stateful_widget(table, chunks[1],
                &mut ratatui::widgets::TableState::new().with_offset(app.wh_scroll));
        }
        WhMode::Create | WhMode::Edit(_) => {
            let is_edit = matches!(app.wh_mode, WhMode::Edit(_));

            let form_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(chunks[1]);

            let name_style = if matches!(app.wh_focus, WhFormFocus::Name) {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else { Style::default() };
            f.render_widget(
                Paragraph::new(app.wh_name.as_str())
                    .block(Block::default().title(" Name ").borders(Borders::ALL).style(name_style)),
                form_chunks[0],
            );

            let loc_style = if matches!(app.wh_focus, WhFormFocus::Location) {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else { Style::default() };
            f.render_widget(
                Paragraph::new(app.wh_location.as_str())
                    .block(Block::default().title(" Location ").borders(Borders::ALL).style(loc_style)),
                form_chunks[1],
            );

            let btn_style = Style::default().fg(Color::Black)
                .bg(if is_edit { Color::Blue } else { Color::Green })
                .add_modifier(Modifier::BOLD);
            f.render_widget(
                Paragraph::new(if is_edit { "[ Update ]" } else { "[ Create ]" })
                    .block(Block::default().borders(Borders::ALL))
                    .style(btn_style).alignment(Alignment::Center),
                form_chunks[2],
            );

            f.render_widget(
                Paragraph::new("Tab: next field  |  Enter: submit  |  Esc: back")
                    .style(Style::default().fg(Color::DarkGray)).alignment(Alignment::Center),
                form_chunks[3],
            );
        }
        WhMode::DeleteConfirm(_, ref name) => {
            let popup = centered_rect(40, 20, chunks[1]);
            f.render_widget(Clear, popup);

            let inner = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(popup);

            let msg = format!(" Delete warehouse \"{name}\"?\n\n This will fail if inventory/transactions still reference it.");
            f.render_widget(
                Paragraph::new(msg.as_str())
                    .block(Block::default().title(" Confirm Delete ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center),
                inner[0],
            );
            f.render_widget(
                Paragraph::new("Enter: confirm  |  Esc: cancel")
                    .style(Style::default().fg(Color::DarkGray)).alignment(Alignment::Center),
                inner[1],
            );
        }
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
