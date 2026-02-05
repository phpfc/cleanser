use crate::types::*;
use anyhow::Result;
use colored::Colorize;
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
use std::time::SystemTime;

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
                    // Cancel - return empty list
                    return Ok(Vec::new());
                }
                KeyCode::Up => {
                    state.move_cursor_up();
                }
                KeyCode::Down => {
                    state.move_cursor_down();
                }
                KeyCode::Char(' ') => {
                    state.toggle_selection();
                }
                KeyCode::Enter => {
                    // Confirm - return selected items
                    if state.selected_count == 0 {
                        // Show message and continue
                        continue;
                    }
                    return Ok(state.get_selected_items().into_iter().cloned().collect());
                }
                _ => {}
            }
        }
    }
}

fn render_ui(f: &mut Frame, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Min(10),         // File list
            Constraint::Length(6),       // Details panel
            Constraint::Length(3),       // Help footer
        ])
        .split(f.size());

    // Render header
    render_header(f, chunks[0], state);

    // Render file list
    render_file_list(f, chunks[1], state);

    // Render details panel
    render_details(f, chunks[2], state);

    // Render help footer
    render_footer(f, chunks[3]);
}

fn render_header(f: &mut Frame, area: Rect, state: &TuiState) {
    let title = format!(
        "Cleanser Interactive Mode - {} items ({} selected, {})",
        state.items.len(),
        state.selected_count,
        format_size(state.total_selected_size, BINARY)
    );

    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn render_file_list(f: &mut Frame, area: Rect, state: &TuiState) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    // Calculate scroll offset to keep cursor visible
    let scroll_offset = if state.cursor_position >= state.scroll_offset + visible_height {
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
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(i, item)| {
            let checkbox = if item.selected { "[✓]" } else { "[ ]" };
            let cursor = if i == state.cursor_position { "> " } else { "  " };
            
            let path_str = item.path.to_string_lossy();
            let size_str = format_size(item.size, BINARY);
            
            // Truncate path if too long
            let max_path_len = area.width.saturating_sub(30) as usize;
            let display_path = if path_str.len() > max_path_len {
                format!("...{}", &path_str[path_str.len() - max_path_len + 3..])
            } else {
                path_str.to_string()
            };

            let line = format!("{}{} {:width$} {:>12}", 
                cursor, checkbox, display_path, size_str,
                width = max_path_len
            );

            let style = if i == state.cursor_position {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if item.selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"));

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

    let paragraph = Paragraph::new(details)
        .block(Block::default().borders(Borders::ALL).title("Details"));

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help_text = "↑/↓: Navigate | Space: Select | Enter: Delete | Esc/q: Exit";
    
    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));

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


pub fn run_interactive_large_file_deletion(
    items: &[CleanableItem],
    dry_run: bool,
) -> Result<()> {
    use std::io::{self, Write};

    let mut prompt = InteractivePrompt::new(items.len());
    
    for (index, item) in items.iter().enumerate() {
        prompt.current_index = index + 1;
        
        // Display file information
        println!("\n{}", "=".repeat(60));
        println!("File {} of {}:", prompt.current_index, prompt.total_files);
        println!("  Path: {}", item.path.display());
        println!("  Size: {}", format_size(item.size, BINARY));
        
        // Get modification time
        if let Ok(metadata) = std::fs::metadata(&item.path) {
            if let Ok(modified) = metadata.modified() {
                println!("  Modified: {}", format_system_time(modified));
            }
        }
        
        println!("{}", "=".repeat(60));
        
        // Prompt for action
        loop {
            print!("\nDelete this file? [d]elete / [s]kip / [q]uit: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice = input.trim().to_lowercase();
            
            match choice.as_str() {
                "d" | "delete" => {
                    if dry_run {
                        println!("{}", "[DRY RUN] Would delete this file".yellow());
                        prompt.deleted_count += 1;
                    } else {
                        match std::fs::remove_file(&item.path) {
                            Ok(_) => {
                                println!("{}", format!("✓ Deleted: {}", item.path.display()).green());
                                prompt.deleted_count += 1;
                            }
                            Err(e) => {
                                eprintln!("{}", format!("✗ Failed to delete: {}", e).red());
                                prompt.skipped_count += 1;
                            }
                        }
                    }
                    break;
                }
                "s" | "skip" => {
                    println!("{}", "Skipped".yellow());
                    prompt.skipped_count += 1;
                    break;
                }
                "q" | "quit" => {
                    println!("\n{}", "Quitting interactive session...".cyan());
                    prompt.display_summary();
                    return Ok(());
                }
                _ => {
                    println!("{}", "Invalid choice. Please enter 'd', 's', or 'q'.".red());
                }
            }
        }
    }
    
    prompt.display_summary();
    Ok(())
}
