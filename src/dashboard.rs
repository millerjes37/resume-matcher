use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame, Terminal,
};
use std::{error::Error, io, fs::File};
use csv::ReaderBuilder;
use serde::Deserialize; // For deserializing CSV data

#[derive(Debug, Deserialize)]
struct ApplicationRecord {
    #[serde(rename = "TrackingNumber")]
    tracking_number: String,
    #[serde(rename = "Company")]
    company: String,
    #[serde(rename = "Role")]
    role: String,
    #[serde(rename = "ApplicationDate")]
    application_date: String,
    #[serde(rename = "ResumeTypPath")]
    resume_typ_path: String,
    #[serde(rename = "CoverLetterTypPath")]
    cover_letter_typ_path: String,
    #[serde(rename = "ResumePdfPath")]
    resume_pdf_path: String,
    #[serde(rename = "CoverLetterPdfPath")]
    cover_letter_pdf_path: String,
    #[serde(rename = "Status")]
    status: String,
}

fn read_application_data() -> Result<Vec<ApplicationRecord>, Box<dyn Error>> {
    let file = File::open("applications.csv")?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
    let mut records = Vec::new();
    for result in rdr.deserialize() {
        let record: ApplicationRecord = result?;
        records.push(record);
    }
    Ok(records)
}

pub fn run_dashboard() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut table_state = TableState::default();
    table_state.select(Some(0)); // Select the first row by default

    loop {
        let records = read_application_data().unwrap_or_else(|_| vec![]); // Handle error if CSV not found/readable
        terminal.draw(|f| ui(f, &records, &mut table_state))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        let i = match table_state.selected() {
                            Some(i) => if records.is_empty() { 0 } else if i >= records.len() - 1 { 0 } else { i + 1 },
                            None => 0,
                        };
                        if !records.is_empty() { table_state.select(Some(i)); } else { table_state.select(None); }
                    }
                    KeyCode::Up => {
                        let i = match table_state.selected() {
                            Some(i) => if records.is_empty() { 0 } else if i == 0 { records.len() - 1 } else { i - 1 },
                            None => 0,
                        };
                        if !records.is_empty() { table_state.select(Some(i)); } else { table_state.select(None); }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui<B: Backend>(f: &mut Frame, records: &[ApplicationRecord], table_state: &mut TableState) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .margin(1)
        .split(f.size());

    let selected_style = Style::default().add_modifier(Modifier::REVERSED).fg(Color::Yellow);
    let normal_style = Style::default().fg(Color::White);
    let header_cells = [
        "Tracking #", "Company", "Role", "Date", "Status",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);

    let rows = records.iter().map(|item| {
        let cells = vec![
            Cell::from(item.tracking_number.clone()),
            Cell::from(item.company.clone()),
            Cell::from(item.role.clone()),
            Cell::from(item.application_date.clone()),
            Cell::from(item.status.clone()),
        ];
        Row::new(cells).style(normal_style) 
    });
    
    let col_widths = vec![
            Constraint::Min(15), // Tracking
            Constraint::Min(20), // Company
            Constraint::Min(20), // Role
            Constraint::Min(12), // Date
            Constraint::Min(10), // Status
        ];

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Applications"))
        .highlight_style(selected_style)
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(table, rects[0], table_state);
}
