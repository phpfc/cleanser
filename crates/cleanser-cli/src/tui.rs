//! Interactive TUI for file selection.

use anyhow::Result;
use cleanser_core::{RiskLevel, ScanResults};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use humansize::{format_size, BINARY};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

// TUI-specific types (not in core)

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub risk_level: RiskLevel,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    SizeDesc,
    SizeAsc,
    DateOldest,
    DateNewest,
    PathAsc,
    PathDesc,
}

impl std::fmt::Display for SortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortMode::SizeDesc => write!(f, "Size ↓ (Heaviest first)"),
            SortMode::SizeAsc => write!(f, "Size ↑ (Lightest first)"),
            SortMode::DateOldest => write!(f, "Date ↑ (Oldest first)"),
            SortMode::DateNewest => write!(f, "Date ↓ (Newest first)"),
            SortMode::PathAsc => write!(f, "Path ↑ (A-Z)"),
            SortMode::PathDesc => write!(f, "Path ↓ (Z-A)"),
        }
    }
}

pub struct TuiState {
    pub items: Vec<FileItem>,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub selected_count: usize,
    pub total_selected_size: u64,
    pub sort_mode: SortMode,
}

impl TuiState {
    pub fn new(mut items: Vec<FileItem>) -> Self {
        items.sort_by(|a, b| b.size.cmp(&a.size));

        Self {
            items,
            cursor_position: 0,
            scroll_offset: 0,
            selected_count: 0,
            total_selected_size: 0,
            sort_mode: SortMode::SizeDesc,
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor_position < self.items.len().saturating_sub(1) {
            self.cursor_position += 1;
        }
    }

    pub fn toggle_selection(&mut self) {
        if let Some(item) = self.items.get_mut(self.cursor_position) {
            item.selected = !item.selected;
            if item.selected {
                self.selected_count += 1;
                self.total_selected_size += item.size;
            } else {
                self.selected_count = self.selected_count.saturating_sub(1);
                self.total_selected_size = self.total_selected_size.saturating_sub(item.size);
            }
        }
    }

    pub fn get_selected_items(&self) -> Vec<&FileItem> {
        self.items.iter().filter(|item| item.selected).collect()
    }

    pub fn set_sort_mode(&mut self, mode: SortMode) {
        self.sort_mode = mode;
        self.sort_items();
    }

    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::SizeDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::DateOldest,
            SortMode::DateOldest => SortMode::DateNewest,
            SortMode::DateNewest => SortMode::PathAsc,
            SortMode::PathAsc => SortMode::PathDesc,
            SortMode::PathDesc => SortMode::SizeDesc,
        };
        self.sort_items();
    }

    fn sort_items(&mut self) {
        match self.sort_mode {
            SortMode::SizeDesc => {
                self.items.sort_by(|a, b| b.size.cmp(&a.size));
            }
            SortMode::SizeAsc => {
                self.items.sort_by(|a, b| a.size.cmp(&b.size));
            }
            SortMode::DateOldest => {
                self.items.sort_by(|a, b| a.modified.cmp(&b.modified));
            }
            SortMode::DateNewest => {
                self.items.sort_by(|a, b| b.modified.cmp(&a.modified));
            }
            SortMode::PathAsc => {
                self.items.sort_by(|a, b| a.path.cmp(&b.path));
            }
            SortMode::PathDesc => {
                self.items.sort_by(|a, b| b.path.cmp(&a.path));
            }
        }

        self.cursor_position = 0;
        self.scroll_offset = 0;
    }
}

pub fn run_interactive_mode(scan_results: &ScanResults) -> Result<Vec<FileItem>> {
    // Convert scan results to FileItems
    let items: Vec<FileItem> = scan_results
        .items
        .iter()
        .map(|item| {
            let modified = std::fs::metadata(&item.path)
                .and_then(|m| m.modified())
                .unwrap_or_else(|_| SystemTime::now());

            FileItem {
                path: item.path.clone(),
                size: item.size,
                modified,
                risk_level: item.risk_level,
                selected: false,
            }
        })
        .collect();

    if items.is_empty() {
        println!("No items to display");
        return Ok(Vec::new());
    }

    let mut state = TuiState::new(items);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the TUI loop
    let result = run_tui_loop(&mut terminal, &mut state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut TuiState,
) -> Result<Vec<FileItem>> {
    loop {
        terminal.draw(|f| render_ui(f, state))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(Vec::new());
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.move_cursor_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    state.move_cursor_down();
                }
                KeyCode::Char(' ') => {
                    state.toggle_selection();
                }
                KeyCode::Enter => {
                    if state.selected_count == 0 {
                        continue;
                    }
                    return Ok(state.get_selected_items().into_iter().cloned().collect());
                }
                KeyCode::Char('s') => {
                    state.set_sort_mode(SortMode::SizeDesc);
                }
                KeyCode::Char('S') => {
                    state.set_sort_mode(SortMode::SizeAsc);
                }
                KeyCode::Char('d') => {
                    state.set_sort_mode(SortMode::DateOldest);
                }
                KeyCode::Char('D') => {
                    state.set_sort_mode(SortMode::DateNewest);
                }
                KeyCode::Char('p') => {
                    state.set_sort_mode(SortMode::PathAsc);
                }
                KeyCode::Char('P') => {
                    state.set_sort_mode(SortMode::PathDesc);
                }
                KeyCode::Tab => {
                    state.cycle_sort_mode();
                }
                _ => {}
            }
        }
    }
}

fn render_ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(6),
            Constraint::Length(4),
        ])
        .split(f.size());

    render_header(f, chunks[0], state);
    render_file_list(f, chunks[1], state);
    render_details(f, chunks[2], state);
    render_footer(f, chunks[3]);
}

fn render_header(f: &mut Frame, area: Rect, state: &TuiState) {
    let title = vec![
        Line::from(vec![Span::styled(
            format!("Cleanser Interactive Mode - {} items", state.items.len()),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!(
                    "{} items ({})",
                    state.selected_count,
                    format_size(state.total_selected_size, BINARY)
                ),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  "),
            Span::styled("Sort: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", state.sort_mode),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let header = Paragraph::new(title).block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn render_file_list(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let visible_height = area.height.saturating_sub(2) as usize;

    state.scroll_offset = if state.cursor_position >= state.scroll_offset + visible_height {
        state.cursor_position.saturating_sub(visible_height - 1)
    } else if state.cursor_position < state.scroll_offset {
        state.cursor_position
    } else {
        state.scroll_offset
    };

    let items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .skip(state.scroll_offset)
        .take(visible_height)
        .map(|(i, item)| {
            let checkbox = if item.selected { "[✓]" } else { "[ ]" };
            let cursor = if i == state.cursor_position {
                "> "
            } else {
                "  "
            };

            let path_str = item.path.to_string_lossy();
            let size_str = format_size(item.size, BINARY);

            let max_path_len = area.width.saturating_sub(30) as usize;
            let display_path = if path_str.len() > max_path_len {
                format!("...{}", &path_str[path_str.len() - max_path_len + 3..])
            } else {
                path_str.to_string()
            };

            let line = format!(
                "{}{} {:width$} {:>12}",
                cursor,
                checkbox,
                display_path,
                size_str,
                width = max_path_len
            );

            let style = if i == state.cursor_position {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if item.selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Files"));

    f.render_widget(list, area);
}

fn render_details(f: &mut Frame, area: Rect, state: &TuiState) {
    let details = if let Some(item) = state.items.get(state.cursor_position) {
        let modified_str = format_system_time(item.modified);

        vec![
            Line::from(vec![
                Span::styled("Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(item.path.to_string_lossy().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Size: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format_size(item.size, BINARY)),
            ]),
            Line::from(vec![
                Span::styled("Modified: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(modified_str),
            ]),
            Line::from(vec![
                Span::styled("Risk: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", item.risk_level)),
            ]),
        ]
    } else {
        vec![Line::from("No item selected")]
    };

    let paragraph =
        Paragraph::new(details).block(Block::default().borders(Borders::ALL).title("Details"));

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("↑/↓/j/k", Style::default().fg(Color::Yellow)),
            Span::raw(": Navigate  "),
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::raw(": Select  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(": Delete  "),
            Span::styled("Esc/q", Style::default().fg(Color::Red)),
            Span::raw(": Exit"),
        ]),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(": Cycle sort  "),
            Span::styled("s/S", Style::default().fg(Color::Cyan)),
            Span::raw(": Size ↓/↑  "),
            Span::styled("d/D", Style::default().fg(Color::Cyan)),
            Span::raw(": Date ↑/↓  "),
            Span::styled("p/P", Style::default().fg(Color::Cyan)),
            Span::raw(": Path A-Z/Z-A"),
        ]),
    ];

    let footer =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(footer, area);
}

fn format_system_time(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    if let Ok(duration) = time.duration_since(UNIX_EPOCH) {
        let secs = duration.as_secs();
        let datetime = chrono::DateTime::from_timestamp(secs as i64, 0);
        if let Some(dt) = datetime {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    "Unknown".to_string()
}
