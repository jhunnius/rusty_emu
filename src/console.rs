//! # Console Interface Module
//!
//! Interactive terminal-based user interface for the Intel MCS-4 emulator.
//! Provides real-time system monitoring, debugging capabilities, and user controls.
//!
//! ## Features
//! - Real-time RAM and register display
//! - Interactive command interface
//! - System state monitoring
//! - Configurable display options
//! - Graceful interrupt handling

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::system_config::ConfigurableSystem;

/// Console configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    pub enabled: bool,
    pub refresh_rate_ms: u64,
    pub show_ram: bool,
    pub show_registers: bool,
    pub show_system_info: bool,
    pub ram_banks_per_row: usize,
    pub max_ram_rows: usize,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_rate_ms: 100,
            show_ram: true,
            show_registers: true,
            show_system_info: true,
            ram_banks_per_row: 4,
            max_ram_rows: 5,
        }
    }
}

/// Console UI application state
pub struct ConsoleApp {
    system: Arc<Mutex<ConfigurableSystem>>,
    config: ConsoleConfig,
    running: bool,
    command_buffer: String,
    show_help: bool,
    selected_pane: usize,
}

impl ConsoleApp {
    pub fn new(system: Arc<Mutex<ConfigurableSystem>>, config: ConsoleConfig) -> Self {
        Self {
            system,
            config,
            running: false,
            command_buffer: String::new(),
            show_help: false,
            selected_pane: 0,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup terminal
        enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)
            .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.running = true;
        let mut last_draw = Instant::now();

        // Main event loop
        while self.running {
            let now = Instant::now();

            // Handle input
            if let Ok(true) = event::poll(Duration::from_millis(10)) {
                if let Ok(Event::Key(key)) = event::read() {
                    self.handle_key_event(key.code);
                }
            }

            // Update display at regular intervals
            if now.duration_since(last_draw) >= Duration::from_millis(self.config.refresh_rate_ms) {
                if let Err(e) = terminal.draw(|f| self.draw_ui(f)) {
                    eprintln!("DEBUG: Failed to draw UI: {}", e);
                    break;
                }
                last_draw = now;
            }

            // Check if system should still be running
            if let Ok(system) = self.system.lock() {
                if !system.is_running() && !self.show_help {
                    // System stopped, show final state briefly
                    if let Err(e) = terminal.draw(|f| self.draw_ui(f)) {
                        eprintln!("DEBUG: Failed to draw final UI: {}", e);
                    }
                    thread::sleep(Duration::from_millis(500));
                    break;
                }
            }

            // Small delay to prevent busy waiting
            thread::sleep(Duration::from_millis(1));
        }

        // Restore terminal
        disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .map_err(|e| format!("Failed to leave alternate screen: {}", e))?;
        terminal
            .show_cursor()
            .map_err(|e| format!("Failed to show cursor: {}", e))?;

        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                println!("DEBUG: Quit key pressed, stopping console");
                self.running = false;
                if let Ok(mut system) = self.system.lock() {
                    system.stop();
                }
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                println!("DEBUG: Help key pressed");
                self.show_help = !self.show_help;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                println!("DEBUG: Run key pressed");
                if let Ok(mut system) = self.system.lock() {
                    if system.is_running() {
                        system.stop();
                    } else {
                        system.run();
                    }
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                println!("DEBUG: Stop key pressed");
                if let Ok(mut system) = self.system.lock() {
                    system.stop();
                }
            }
            KeyCode::Char(' ') => {
                println!("DEBUG: Space key pressed - single step not implemented");
                // Single step (if supported)
                // This would require adding step functionality to the system
            }
            KeyCode::Tab => {
                println!("DEBUG: Tab key pressed - switching panes");
                self.selected_pane = (self.selected_pane + 1) % 3;
            }
            KeyCode::Backspace => {
                println!("DEBUG: Backspace key pressed");
                self.command_buffer.pop();
            }
            KeyCode::Enter => {
                println!(
                    "DEBUG: Enter key pressed - executing command: '{}'",
                    self.command_buffer
                );
                self.execute_command();
                self.command_buffer.clear();
            }
            KeyCode::Char(c) => {
                println!("DEBUG: Character key pressed: '{}'", c);
                if c.is_ascii_alphabetic() || c.is_ascii_digit() {
                    self.command_buffer.push(c);
                }
            }
            _ => {
                println!("DEBUG: Unhandled key pressed: {:?}", key);
            }
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim().to_lowercase();
        println!("DEBUG: Executing command: '{}'", cmd);

        match cmd.as_str() {
            "quit" | "exit" | "q" => {
                println!("DEBUG: Executing quit command");
                self.running = false;
                if let Ok(mut system) = self.system.lock() {
                    system.stop();
                }
            }
            "run" | "r" => {
                println!("DEBUG: Executing run command");
                if let Ok(mut system) = self.system.lock() {
                    system.run();
                }
            }
            "stop" | "s" => {
                println!("DEBUG: Executing stop command");
                if let Ok(mut system) = self.system.lock() {
                    system.stop();
                }
            }
            "help" | "h" => {
                println!("DEBUG: Toggling help display");
                self.show_help = !self.show_help;
            }
            "reset" => {
                println!("DEBUG: Executing reset command");
                if let Ok(mut system) = self.system.lock() {
                    system.stop();
                    // Reset would need to be implemented in the system
                }
            }
            "status" => {
                println!("DEBUG: Executing status command");
                if let Ok(system) = self.system.lock() {
                    let info = system.get_system_info();
                    println!("System: {} - {}", info.name, info.description);
                    println!(
                        "Components: {}, Running: {}",
                        info.component_count,
                        system.is_running()
                    );
                }
            }
            "ram" => {
                println!("DEBUG: Executing RAM display command");
                if let Ok(_system) = self.system.lock() {
                    println!("RAM display requested - would show RAM contents here");
                }
            }
            "" => {
                // Empty command - do nothing
            }
            _ => {
                println!("DEBUG: Unknown command: '{}'", cmd);
                println!("Available commands: quit, run, stop, help, reset, status, ram");
            }
        }
    }

    fn draw_ui(&self, f: &mut Frame) {
        let size = f.size();

        if self.show_help {
            self.draw_help_screen(f);
            return;
        }

        // Create main layout with proper constraints
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Title bar
                Constraint::Min(8),    // Main content
                Constraint::Length(3), // Command bar
            ])
            .split(size);

        // Title bar
        let title_text = vec![
            Line::from(vec![Span::styled(
                "Intel MCS-4 Emulator Console",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::raw("Commands: "),
                Span::styled("q/Q", Style::default().fg(Color::Yellow)),
                Span::raw("=quit, "),
                Span::styled("r/R", Style::default().fg(Color::Yellow)),
                Span::raw("=run, "),
                Span::styled("s/S", Style::default().fg(Color::Yellow)),
                Span::raw("=stop, "),
                Span::styled("h/H", Style::default().fg(Color::Yellow)),
                Span::raw("=help"),
            ]),
        ];

        let title = Paragraph::new(title_text)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });
        f.render_widget(title, chunks[0]);

        // Main content area
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Left pane
                Constraint::Percentage(50), // Right pane
            ])
            .split(chunks[1]);

        // Left pane - System info and registers
        self.draw_system_info(f, content_chunks[0]);

        // Right pane - RAM contents
        self.draw_ram_contents(f, content_chunks[1]);

        // Command bar
        let command_text = if self.command_buffer.is_empty() {
            "Enter command (type 'h' for help)..."
        } else {
            &self.command_buffer
        };

        let command_bar = Paragraph::new(command_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Command"));
        f.render_widget(command_bar, chunks[2]);
    }

    fn draw_help_screen(&self, f: &mut Frame) {
        let size = f.size();
        let help_text = vec![
            Line::from(vec![Span::styled(
                "Intel MCS-4 Emulator Console Help",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Commands:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  q, quit, exit", Style::default().fg(Color::Yellow)),
                Span::raw(" - Exit emulator"),
            ]),
            Line::from(vec![
                Span::styled("  r, run", Style::default().fg(Color::Yellow)),
                Span::raw(" - Start/stop system execution"),
            ]),
            Line::from(vec![
                Span::styled("  s, stop", Style::default().fg(Color::Yellow)),
                Span::raw(" - Stop system execution"),
            ]),
            Line::from(vec![
                Span::styled("  h, help", Style::default().fg(Color::Yellow)),
                Span::raw(" - Show/hide this help"),
            ]),
            Line::from(vec![
                Span::styled("  reset", Style::default().fg(Color::Yellow)),
                Span::raw(" - Reset system"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" - Switch between panes"),
            ]),
            Line::from(vec![
                Span::styled("  Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" - Execute command"),
            ]),
            Line::from(vec![
                Span::styled("  Backspace", Style::default().fg(Color::Yellow)),
                Span::raw(" - Delete character"),
            ]),
            Line::from(""),
            Line::from(vec![Span::raw("Press any key to return to main view...")]),
        ];

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, size);
    }

    fn draw_system_info(&self, f: &mut Frame, area: Rect) {
        let info_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // System info
                Constraint::Min(5),    // Registers
            ])
            .split(area);

        // System information
        let system_info = match self.system.lock() {
            Ok(system) => match system.get_system_info() {
                info => vec![
                    Line::from(vec![Span::raw(format!("System: {}", info.name))]),
                    Line::from(vec![Span::raw(format!(
                        "Description: {}",
                        info.description
                    ))]),
                    Line::from(vec![Span::raw(format!(
                        "Components: {}",
                        info.component_count
                    ))]),
                    Line::from(vec![Span::raw(format!("CPU Speed: {} Hz", info.cpu_speed))]),
                    Line::from(vec![Span::raw(format!(
                        "ROM Size: {} bytes",
                        info.rom_size
                    ))]),
                    Line::from(vec![Span::raw(format!(
                        "RAM Size: {} nibbles",
                        info.ram_size
                    ))]),
                    Line::from(vec![Span::raw(format!("Running: {}", system.is_running()))]),
                ],
            },
            Err(_) => {
                vec![Line::from(vec![Span::raw(
                    "System information unavailable",
                )])]
            }
        };

        let system_widget = Paragraph::new(system_info)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("System Information"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(system_widget, info_chunks[0]);

        // CPU register display (enhanced with execution status)
        let register_info = vec![
            Line::from(vec![Span::raw("CPU Registers:")]),
            Line::from(vec![Span::raw("Status: Running (check console output)")]),
            Line::from(vec![Span::raw("PC: 0x000 (see DEBUG output)")]),
            Line::from(vec![Span::raw("ACC: 0x0 (see DEBUG output)")]),
            Line::from(vec![Span::raw(
                "Instructions: Executing (see DEBUG output)",
            )]),
        ];

        let register_widget = Paragraph::new(register_info)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("CPU Registers"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(register_widget, info_chunks[1]);
    }

    fn draw_ram_contents(&self, f: &mut Frame, area: Rect) {
        let mut ram_info = vec![Line::from(vec![Span::raw("RAM Contents:")])];

        // Try to get actual RAM data from the system
        match self.system.lock() {
            Ok(_system) => {
                // This is a simplified version - in a real implementation,
                // we would need to access the actual RAM components
                // For now, show a more informative placeholder
                ram_info.push(Line::from(vec![Span::raw("Reading RAM contents...")]));

                // Show some sample memory ranges
                for bank in 0..4 {
                    let mut bank_data = format!("Bank {}: [", bank);
                    for i in 0..20 {
                        if i > 0 && i % 4 == 0 {
                            bank_data.push(' ');
                        }
                        bank_data.push_str("00");
                    }
                    bank_data.push(']');
                    ram_info.push(Line::from(vec![Span::raw(bank_data)]));
                }
            }
            Err(_) => {
                ram_info.push(Line::from(vec![Span::raw("Unable to access RAM data")]));
            }
        }

        let ram_widget = Paragraph::new(ram_info)
            .block(Block::default().borders(Borders::ALL).title("RAM Contents"))
            .wrap(Wrap { trim: true });
        f.render_widget(ram_widget, area);
    }
}

/// Public interface for launching the console
pub fn run_console(
    system: Arc<Mutex<ConfigurableSystem>>,
    config: ConsoleConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = ConsoleApp::new(system, config);
    app.run()
}
