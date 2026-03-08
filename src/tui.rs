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
use std::path::PathBuf;
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
                    // Confirm - return selected items
                    if state.selected_count == 0 {
                        // Show message and continue
                        continue;
                    }
                    return Ok(state.get_selected_items().into_iter().cloned().collect());
                }
                // Sort by size (descending - heaviest first)
                KeyCode::Char('s') => {
                    state.set_sort_mode(SortMode::SizeDesc);
                }
                // Sort by size (ascending - lightest first)
                KeyCode::Char('S') => {
                    state.set_sort_mode(SortMode::SizeAsc);
                }
                // Sort by date (oldest first)
                KeyCode::Char('d') => {
                    state.set_sort_mode(SortMode::DateOldest);
                }
                // Sort by date (newest first)
                KeyCode::Char('D') => {
                    state.set_sort_mode(SortMode::DateNewest);
                }
                // Sort by path (A-Z)
                KeyCode::Char('p') => {
                    state.set_sort_mode(SortMode::PathAsc);
                }
                // Sort by path (Z-A)
                KeyCode::Char('P') => {
                    state.set_sort_mode(SortMode::PathDesc);
                }
                // Cycle through sort modes
                KeyCode::Tab => {
                    state.cycle_sort_mode();
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
            Constraint::Length(4), // Header (expanded to 4 for 2 lines)
            Constraint::Min(10),   // File list
            Constraint::Length(6), // Details panel
            Constraint::Length(4), // Help footer (expanded to 4 for 2 lines)
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
            let cursor = if i == state.cursor_position {
                "> "
            } else {
                "  "
            };

            let path_str = item.path.to_string_lossy();
            let size_str = format_size(item.size, BINARY);

            // Truncate path if too long
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

#[allow(dead_code)]
pub fn run_interactive_large_file_deletion(
    items: &[CleanableItem],
    dry_run: bool,
    deleted_paths: &mut Vec<PathBuf>,
) -> Result<()> {
    use std::io::{self, Write};

    let mut prompt = InteractivePrompt::new(items.len());

    for (index, item) in items.iter().enumerate() {
        prompt.current_index = index + 1;

        // Display file information in a cleaner format
        println!(
            "\n{}",
            "============================================================".cyan()
        );
        println!(
            "{}",
            format!("File {} of {}:", prompt.current_index, prompt.total_files).bold()
        );
        println!("  {}: {}", "Path".bold(), item.path.display());
        println!(
            "  {}: {}",
            "Size".bold(),
            format_size(item.size, BINARY).green()
        );
        println!("  {}: {}", "Category".bold(), item.category);
        println!(
            "  {}: {}",
            "Risk Level".bold(),
            match item.risk_level {
                RiskLevel::Safe => format!("{}", item.risk_level).green(),
                RiskLevel::Moderate => format!("{}", item.risk_level).yellow(),
                RiskLevel::Risky => format!("{}", item.risk_level).red(),
            }
        );

        // Get modification time
        if let Ok(metadata) = std::fs::metadata(&item.path) {
            if let Ok(modified) = metadata.modified() {
                println!("  {}: {}", "Modified".bold(), format_system_time(modified));
            }
        }

        println!(
            "{}",
            "============================================================".cyan()
        );

        // Prompt for action
        loop {
            print!("\n{}", "Delete this file? ".cyan());
            print!("{}", "[d]".green());
            print!("elete / ");
            print!("{}", "[s]".yellow());
            print!("kip / ");
            print!("{}", "[q]".red());
            print!("uit: ");
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
                        // Handle both files and directories
                        let result = if item.path.is_dir() {
                            std::fs::remove_dir_all(&item.path)
                        } else {
                            std::fs::remove_file(&item.path)
                        };

                        match result {
                            Ok(_) => {
                                println!(
                                    "{}",
                                    format!("✓ Deleted: {}", item.path.display()).green()
                                );
                                prompt.deleted_count += 1;
                                deleted_paths.push(item.path.clone());
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
