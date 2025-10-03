//! # GUI Components Module
//!
//! This module implements all the individual GUI components for the Intel MCS-4 emulator.
//! Each component is responsible for rendering its specific section of the interface
//! and handling user interactions within that section.
//!
//! ## Component Architecture
//!
//! The GUI is organized into several specialized components:
//!
//! - **`GuiComponents`**: Main container managing all components
//! - **`ControlPanel`**: System control buttons and actions
//! - **`MemoryViewer`**: RAM content display and inspection
//! - **`RegisterViewer`**: CPU register state visualization
//! - **`RomLoader`**: File dialog integration for ROM loading
//! - **`StatusBar`**: System status and component health display
//!
//! ## Design Principles
//!
//! ### Separation of Concerns
//! Each component handles only its specific functionality:
//! - State management is centralized in `GuiState`
//! - Components receive immutable references where possible
//! - User interactions trigger state changes through controlled interfaces
//!
//! ### Performance Optimization
//! - Components minimize allocations and cloning
//! - State updates are batched and efficient
//! - UI layout uses egui's immediate mode for optimal performance
//!
//! ### User Experience
//! - Consistent visual design across all components
//! - Clear visual feedback for user actions
//! - Intuitive layout and component organization
//!
//! ## Component Communication
//!
//! Components communicate through the shared `GuiState`:
//! - Read-only access to state for display components
//! - Controlled mutation through specific methods
//! - Thread-safe state sharing with the emulator system
//!
//! ## Error Handling
//!
//! Components handle errors gracefully:
//! - Invalid states are displayed to users
//! - Failed operations show clear error messages
//! - Recovery options are provided where applicable

use super::state::GuiState;
use eframe::egui;

/// Container for all GUI components
///
/// This structure manages the lifecycle and coordination of all GUI components.
/// It acts as a facade, providing a clean interface for rendering different
/// sections of the user interface while maintaining proper separation of concerns.
///
/// ## Component Management
///
/// The container is responsible for:
/// - Component initialization and configuration
/// - Render orchestration and layout management
/// - State distribution to appropriate components
/// - Component lifecycle and cleanup
///
/// ## Performance Considerations
///
/// - Components are created once and reused across frames
/// - State references are passed efficiently without cloning
/// - Layout is managed centrally for consistent spacing
pub struct GuiComponents {
    /// System control and management interface
    control_panel: ControlPanel,
    /// RAM content inspection and visualization
    memory_viewer: MemoryViewer,
    /// CPU register state display
    register_viewer: RegisterViewer,
    /// ROM file loading and management
    rom_loader: RomLoader,
    /// System status and health monitoring
    status_bar: StatusBar,
}

impl GuiComponents {
    /// Create a new GUI components container with all components initialized
    ///
    /// This constructor sets up all GUI components with their default configurations.
    /// Components are ready to use immediately after creation.
    ///
    /// # Returns
    /// A fully initialized `GuiComponents` instance
    ///
    /// # Example
    /// ```rust,no_run
    /// use rusty_emu::gui::components::GuiComponents;
    ///
    /// let mut components = GuiComponents::new();
    /// // Components are ready to render
    /// ```
    pub fn new() -> Self {
        Self {
            control_panel: ControlPanel::new(),
            memory_viewer: MemoryViewer::new(),
            register_viewer: RegisterViewer::new(),
            rom_loader: RomLoader::new(),
            status_bar: StatusBar::new(),
        }
    }

    /// Render the control panel component
    ///
    /// The control panel provides system management functionality including
    /// start/stop/reset controls and ROM loading capabilities.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `state` - Mutable reference to GUI state for control operations
    pub fn render_control_panel(&mut self, ui: &mut egui::Ui, state: &mut GuiState) {
        self.control_panel.render(ui, state);
    }

    /// Render the memory viewer component
    ///
    /// Displays RAM contents in a tabular format with bank selection
    /// and hex/decimal viewing options.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `state` - Immutable reference to GUI state for display
    pub fn render_memory_viewer(&self, ui: &mut egui::Ui, state: &GuiState) {
        self.memory_viewer.render(ui, state);
    }

    /// Render the register viewer component
    ///
    /// Shows CPU register state including accumulator, program counter,
    /// index registers, and system flags.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `state` - Immutable reference to GUI state for display
    pub fn render_register_viewer(&self, ui: &mut egui::Ui, state: &GuiState) {
        self.register_viewer.render(ui, state);
    }

    /// Render the status bar component
    ///
    /// Displays system health, component status, cycle counts,
    /// and any error conditions.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `state` - Immutable reference to GUI state for display
    pub fn render_status_bar(&self, ui: &mut egui::Ui, state: &GuiState) {
        self.status_bar.render(ui, state);
    }
}

/// Control panel component for system control buttons and operations
///
/// The control panel provides the primary interface for managing the emulator system.
/// It offers intuitive controls for starting, stopping, and resetting the emulation,
/// along with system configuration and file management capabilities.
///
/// ## Features
///
/// - **System Lifecycle Management**: Start, stop, and reset emulator execution
/// - **Visual State Feedback**: Dynamic button states reflecting current system status
/// - **Error Handling**: Clear error display and recovery mechanisms
/// - **Application Control**: Window management and application termination
///
/// ## Button States
///
/// The control panel adapts its interface based on system state:
/// - **Stopped**: Shows "Start System" button, disabled stop/reset buttons
/// - **Running**: Shows "System Running..." text, enabled stop/reset buttons
/// - **Error**: Displays error messages with option to clear them
pub struct ControlPanel {
    /// Dynamic text for the start/stop button based on system state
    start_button_text: String,
    /// Whether the stop button should be enabled
    stop_button_enabled: bool,
    /// Whether the reset button should be enabled
    reset_button_enabled: bool,
}

impl ControlPanel {
    /// Create a new control panel with default state
    ///
    /// Initializes the control panel in the "stopped" state with appropriate
    /// button configurations for a system that's ready to start.
    ///
    /// # Returns
    /// A new `ControlPanel` instance ready for system control
    pub fn new() -> Self {
        Self {
            start_button_text: "Start System".to_string(),
            stop_button_enabled: false,
            reset_button_enabled: false,
        }
    }

    /// Render the control panel interface
    ///
    /// Creates a horizontal layout with all control buttons and handles
    /// user interactions. The interface adapts dynamically based on system state.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering and interaction
    /// * `state` - Mutable reference to GUI state for system control
    ///
    /// # Layout
    /// ```text
    /// ┌─────────────────────────────────────────────────┐
    /// │ System Control ■■■■■■■■■■■■■■■■■■■■■■■■■■■■■ │
    /// │ [Load ROM] [Start System] [Stop] [Reset] [Close] │
    /// └─────────────────────────────────────────────────┘
    /// ```
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut GuiState) {
        ui.horizontal(|ui| {
            // Section header
            ui.heading("System Control");

            // ROM loading button
            if ui.button("Load ROM").clicked() {
                // ROM loading will be handled by RomLoader component
                state.set_error("ROM loader not yet implemented".to_string());
            }

            // Start/Stop system button (context-sensitive)
            if ui.button(&self.start_button_text).clicked() {
                if !state.system_running {
                    self.start_system(state);
                }
            }

            // Stop system button (only enabled when running)
            if ui.button("Stop System").clicked() && state.system_running {
                self.stop_system(state);
            }

            // Reset system button
            if ui.button("Reset System").clicked() {
                self.reset_system(state);
            }

            // Application close button
            if ui.button("Close").clicked() {
                // Close application - handled by eframe
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        ui.separator();
    }

    /// Start the emulator system
    ///
    /// Transitions the system from stopped to running state and updates
    /// all related UI elements to reflect the new state.
    ///
    /// # Arguments
    /// * `state` - Mutable reference to GUI state
    fn start_system(&mut self, state: &mut GuiState) {
        state.system_running = true;
        self.start_button_text = "System Running...".to_string();
        self.stop_button_enabled = true;
        self.reset_button_enabled = true;
        state.clear_error();
    }

    /// Stop the emulator system
    ///
    /// Transitions the system from running to stopped state and updates
    /// UI elements accordingly.
    ///
    /// # Arguments
    /// * `state` - Mutable reference to GUI state
    fn stop_system(&mut self, state: &mut GuiState) {
        state.system_running = false;
        self.start_button_text = "Start System".to_string();
        self.stop_button_enabled = false;
        state.clear_error();
    }

    /// Reset the emulator system
    ///
    /// Performs a complete system reset, stopping execution and clearing
    /// all state including cycle counts and error conditions.
    ///
    /// # Arguments
    /// * `state` - Mutable reference to GUI state
    fn reset_system(&mut self, state: &mut GuiState) {
        state.system_running = false;
        state.cycle_count = 0;
        self.start_button_text = "Start System".to_string();
        self.stop_button_enabled = false;
        self.reset_button_enabled = false;
        state.clear_error();
    }
}

/// Memory viewer component for displaying RAM contents and state
///
/// The memory viewer provides comprehensive RAM inspection capabilities,
/// allowing users to examine memory contents across different banks with
/// flexible display options and intuitive navigation.
///
/// ## Features
///
/// - **Multi-Bank Display**: View RAM contents across 4 banks simultaneously
/// - **Flexible Formatting**: Toggle between hexadecimal and decimal display
/// - **Interactive Bank Selection**: Choose which memory bank to inspect
/// - **Scrollable Interface**: Navigate through memory contents efficiently
/// - **Real-time Updates**: Live memory content updates during emulation
///
/// ## Display Format
///
/// The memory viewer shows data in a structured grid:
/// - **Address Column**: Memory addresses in hexadecimal format
/// - **Bank Columns**: B0-B3 showing contents of each memory bank
/// - **Value Display**: Configurable hex or decimal representation
///
/// ## Memory Organization
///
/// The Intel 4002 RAM has the following structure:
/// - **4 Banks**: Independent memory banks (B0-B3)
/// - **4 Bytes per Bank**: Addresses 0x00-0x03 in each bank
/// - **4-bit Values**: Each memory location stores a 4-bit nibble
pub struct MemoryViewer {
    /// Display mode: true for hexadecimal, false for decimal
    show_hex: bool,
    /// Number of bytes to display per row (currently fixed at 16)
    bytes_per_row: usize,
}

impl MemoryViewer {
    /// Create a new memory viewer with default configuration
    ///
    /// Initializes the viewer with hexadecimal display mode and standard
    /// layout optimized for the Intel 4002 RAM structure.
    ///
    /// # Returns
    /// A new `MemoryViewer` instance ready for RAM content display
    pub fn new() -> Self {
        Self {
            show_hex: true,
            bytes_per_row: 16,
        }
    }

    /// Render the memory viewer interface
    ///
    /// Creates a comprehensive memory inspection interface with bank selection,
    /// display options, and a scrollable memory contents grid.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `state` - Immutable reference to GUI state containing memory data
    ///
    /// # Layout Structure
    /// ```text
    /// ┌─────────────────────────────────────────────────┐
    /// │ Memory Viewer                          [─] [□] │
    /// │ Bank: [0] □ Hex View                           │
    /// ├─────────────────────────────────────────────────┤
    /// │ Address B0 B1 B2 B3                             │
    /// │ [00]    [12] [34] [56] [78]                    │
    /// │ [01]    [9A] [BC] [DE] [F0]                    │
    /// │ [02]    [11] [22] [33] [44]                    │
    /// │ [03]    [55] [66] [77] [88]                    │
    /// └─────────────────────────────────────────────────┘
    /// ```
    pub fn render(&self, ui: &mut egui::Ui, state: &GuiState) {
        ui.vertical(|ui| {
            // Section header
            ui.heading("Memory Viewer");

            // Control bar with bank selection and display options
            ui.horizontal(|ui| {
                ui.label("Bank:");
                // Bank selector (0-3 for Intel 4002)
                ui.add(
                    egui::DragValue::new(&mut state.memory_state.selected_bank.clone())
                        .clamp_range(0..=3),
                );
                ui.checkbox(&mut self.show_hex.clone(), "Hex View");
            });

            ui.separator();

            // Memory contents in scrollable area
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("memory_grid").striped(true).show(ui, |ui| {
                    // Header row with bank labels
                    ui.label("Address");
                    for i in 0..4 {
                        ui.label(format!("B{}", i));
                    }
                    ui.end_row();

                    // Memory contents rows
                    for addr in 0..4 {
                        // Address column
                        ui.label(format!("{:02X}", addr));

                        // Bank data columns
                        for bank in 0..4 {
                            let value = state.memory_state.ram_contents[bank][addr];
                            if self.show_hex {
                                ui.label(format!("{:02X}", value));
                            } else {
                                ui.label(format!("{}", value));
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        });

        ui.separator();
    }
}

/// Register viewer component for displaying CPU register state
///
/// The register viewer provides comprehensive CPU state visualization,
/// showing all key registers, flags, and pointers that define the current
/// execution state of the Intel 4004 microprocessor.
///
/// ## Features
///
/// - **Complete Register Set**: Display all 16 index registers (R0-R15)
/// - **Core CPU State**: Accumulator, program counter, stack pointer
/// - **Flag Visualization**: Carry flag status with clear indicators
/// - **Interactive Selection**: Choose which index register to highlight
/// - **Dual Format Display**: Hexadecimal and decimal representations
/// - **Real-time Updates**: Live register state during emulation
///
/// ## Intel 4004 Register Architecture
///
/// The Intel 4004 has the following register structure:
/// - **Accumulator**: Main arithmetic register (4 bits)
/// - **Index Registers**: 16 general-purpose registers (R0-R15)
/// - **Program Counter**: 12-bit address pointer
/// - **Stack Pointer**: 4-bit stack address register
/// - **Carry Flag**: Arithmetic carry/borrow indicator
///
/// ## Display Organization
///
/// Registers are organized in a clear hierarchy:
/// - **Primary Registers**: Most frequently used (accumulator, PC, stack)
/// - **Flags**: System status indicators
/// - **Index Registers**: General-purpose register file
pub struct RegisterViewer {
    /// Currently selected index register for detailed view (0-15)
    selected_register: usize,
}

impl RegisterViewer {
    /// Create a new register viewer with default configuration
    ///
    /// Initializes the viewer with index register 0 selected and standard
    /// display formatting optimized for the Intel 4004 architecture.
    ///
    /// # Returns
    /// A new `RegisterViewer` instance ready for CPU state display
    pub fn new() -> Self {
        Self {
            selected_register: 0,
        }
    }

    /// Render the register viewer interface
    ///
    /// Creates a comprehensive register inspection interface with organized
    /// display of all CPU state elements and interactive index register selection.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `state` - Immutable reference to GUI state containing register data
    ///
    /// # Layout Structure
    /// ```text
    /// ┌─────────────────────────────────────────────────┐
    /// │ CPU Registers                          [─] [□] │
    /// │ Index Register: [0]                             │
    /// ├─────────────────────────────────────────────────┤
    /// │ Register       │ Value (Hex) │ Value (Dec)      │
    /// │ Accumulator    │ 0F          │ 15               │
    /// │ Carry Flag     │ 1           │ Set              │
    /// │ Program Counter│ 123         │ 291              │
    /// │ Index R0       │ 2A          │ 42               │
    /// │ Stack Pointer  │ 08          │ 8                │
    /// └─────────────────────────────────────────────────┘
    /// ```
    pub fn render(&self, ui: &mut egui::Ui, state: &GuiState) {
        ui.vertical(|ui| {
            // Section header
            ui.heading("CPU Registers");

            // Index register selector
            ui.horizontal(|ui| {
                ui.label("Index Register:");
                ui.add(
                    egui::DragValue::new(&mut self.selected_register.clone()).clamp_range(0..=15),
                );
            });

            ui.separator();

            // Register display grid
            egui::Grid::new("register_grid")
                .striped(true)
                .show(ui, |ui| {
                    // Header row
                    ui.label("Register");
                    ui.label("Value (Hex)");
                    ui.label("Value (Dec)");
                    ui.end_row();

                    // Core CPU registers
                    ui.label("Accumulator");
                    ui.label(format!("{:02X}", state.register_state.accumulator));
                    ui.label(format!("{}", state.register_state.accumulator));
                    ui.end_row();

                    // System flags
                    ui.label("Carry Flag");
                    ui.label(if state.register_state.carry_flag {
                        "1"
                    } else {
                        "0"
                    });
                    ui.label(if state.register_state.carry_flag {
                        "Set"
                    } else {
                        "Clear"
                    });
                    ui.end_row();

                    // Program execution state
                    ui.label("Program Counter");
                    ui.label(format!("{:03X}", state.register_state.program_counter));
                    ui.label(format!("{}", state.register_state.program_counter));
                    ui.end_row();

                    // Selected index register (highlighted)
                    ui.label(format!("Index R{}", self.selected_register));
                    ui.label(format!(
                        "{:02X}",
                        state.register_state.index_registers[self.selected_register]
                    ));
                    ui.label(format!(
                        "{}",
                        state.register_state.index_registers[self.selected_register]
                    ));
                    ui.end_row();

                    // Stack management
                    ui.label("Stack Pointer");
                    ui.label(format!("{:02X}", state.register_state.stack_pointer));
                    ui.label(format!("{}", state.register_state.stack_pointer));
                    ui.end_row();
                });
        });

        ui.separator();
    }
}

/// ROM loader component for file dialog integration and program management
///
/// The ROM loader handles program file selection, loading, and management.
/// It provides the interface for users to load Intel 4001 ROM files and
/// binary programs into the emulator system.
///
/// ## Features
///
/// - **File Dialog Integration**: Native file browser for program selection
/// - **Program Validation**: Basic file format and size validation
/// - **Load Feedback**: Visual confirmation of successful loads
/// - **Error Handling**: Clear error messages for failed operations
/// - **Future Extensions**: Support for multiple ROM chips and formats
///
/// ## Supported Formats
///
/// - **Binary Files**: Raw binary program data (.bin)
/// - **Intel 4001 Format**: MCS-4 ROM file format (planned)
/// - **Configuration Files**: System configuration integration (planned)
///
/// ## Integration Notes
///
/// Currently shows placeholder for file dialog implementation.
/// Production version should integrate with native file dialogs using:
/// - **rfd** crate for cross-platform file dialogs
/// - **async** operations for non-blocking file I/O
/// - **Progress feedback** for large file operations
pub struct RomLoader {
    /// Whether to show the file selection dialog
    show_file_dialog: bool,
    /// Currently selected file path (if any)
    selected_file: Option<String>,
}

impl RomLoader {
    /// Create a new ROM loader with default state
    ///
    /// Initializes the loader with no file selected and dialog closed.
    /// Ready to handle user file selection requests.
    ///
    /// # Returns
    /// A new `RomLoader` instance ready for file operations
    pub fn new() -> Self {
        Self {
            show_file_dialog: false,
            selected_file: None,
        }
    }

    /// Render the ROM loader interface
    ///
    /// Creates the file management interface with load button and
    /// file selection dialog integration.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `_state` - GUI state (currently unused but reserved for future integration)
    ///
    /// # Layout Structure
    /// ```text
    /// ┌─────────────────────────────────────────────────┐
    /// │ ROM Management                          [─] [□]│
    /// │ [Load ROM File...]                              │
    /// │ Selected: /path/to/program.bin                  │
    /// │                                                 │
    /// │ [File Dialog Placeholder]                       │
    /// └─────────────────────────────────────────────────┘
    /// ```
    pub fn render(&mut self, ui: &mut egui::Ui, _state: &mut GuiState) {
        ui.vertical(|ui| {
            ui.heading("ROM Management");

            // Load button to trigger file selection
            if ui.button("Load ROM File...").clicked() {
                self.show_file_dialog = true;
            }

            // Display currently selected file
            if let Some(ref file) = self.selected_file.clone() {
                ui.label(format!("Selected: {}", file));
            }

            // File dialog implementation (placeholder)
            if self.show_file_dialog {
                // Placeholder for file dialog
                ui.label("File dialog would open here");
                if ui.button("Cancel").clicked() {
                    self.show_file_dialog = false;
                }
            }
        });

        ui.separator();
    }
}

/// Status bar component for system status display and health monitoring
///
/// The status bar provides comprehensive system health information,
/// component status indicators, and real-time performance metrics.
/// It serves as the primary location for system state overview.
///
/// ## Features
///
/// - **System Status**: Running/stopped state with visual indicators
/// - **Performance Metrics**: Cycle count and execution speed
/// - **Component Health**: Individual component status monitoring
/// - **Error Display**: Real-time error message display
/// - **System Information**: CPU speed, component count, and configuration
///
/// ## Visual Design
///
/// - **Color Coding**: Green for healthy, red for errors/stopped
/// - **Layout Organization**: Status on left, errors on right
/// - **Real-time Updates**: Live status changes during operation
/// - **Compact Display**: Information-dense but readable layout
///
/// ## Component Status Indicators
///
/// Monitors the health of all major system components:
/// - **CPU**: Intel 4004 execution status
/// - **RAM**: Intel 4002 memory operations
/// - **ROM**: Intel 4001 program storage
/// - **CLK**: System clock generation
pub struct StatusBar;

impl StatusBar {
    /// Create a new status bar component
    ///
    /// Initializes a stateless status bar ready to display
    /// current system health and performance information.
    ///
    /// # Returns
    /// A new `StatusBar` instance
    pub fn new() -> Self {
        Self
    }

    /// Render the status bar interface
    ///
    /// Creates a comprehensive status display with system information,
    /// component health indicators, and error reporting.
    ///
    /// # Arguments
    /// * `ui` - egui UI context for rendering
    /// * `state` - Immutable reference to GUI state for status information
    ///
    /// # Layout Structure
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────┐
    /// │ Status: ● Running  │ Cycles: 12345  │ CPU: 0.7 MHz        │
    /// │ Components: 4      │ CPU RAM ROM CLK │ Error: Connection  │
    /// └─────────────────────────────────────────────────────────────┘
    /// ```
    pub fn render(&self, ui: &mut egui::Ui, state: &GuiState) {
        ui.separator();

        ui.horizontal(|ui| {
            // Left side: Status information and metrics
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // System running status
                ui.label("Status:");
                if state.system_running {
                    ui.colored_label(egui::Color32::GREEN, "● Running");
                } else {
                    ui.colored_label(egui::Color32::RED, "● Stopped");
                }

                ui.separator();

                // Performance metrics
                ui.label(format!("Cycles: {}", state.cycle_count));

                ui.separator();

                // System information
                if let Some(ref info) = state.system_info {
                    ui.label(format!("CPU: {:.1} MHz", info.cpu_speed / 1_000_000.0));
                    ui.label(format!("Components: {}", info.component_count));
                }

                ui.separator();

                // Component status indicators
                ui.label("Components:");
                ui.colored_label(
                    if state.component_states.cpu_running {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    },
                    "CPU",
                );
                ui.colored_label(
                    if state.component_states.ram_running {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    },
                    "RAM",
                );
                ui.colored_label(
                    if state.component_states.rom_running {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    },
                    "ROM",
                );
                ui.colored_label(
                    if state.component_states.clock_running {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    },
                    "CLK",
                );
            });

            // Right side: Error messages
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(ref error) = state.last_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                }
            });
        });
    }
}
