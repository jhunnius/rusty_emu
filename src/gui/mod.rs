//! # GUI Module
//!
//! This module provides a complete graphical user interface for the Intel MCS-4 emulator.
//! It includes state management, component rendering, and system integration.

pub mod components;
pub mod state;

use crate::system_config::ConfigurableSystem;
use eframe::egui;
use std::sync::{Arc, Mutex};

/// Main GUI application structure
///
/// This structure manages the entire GUI application lifecycle, including:
/// - System integration and control
/// - State management and updates
/// - Component rendering and interaction
/// - Error handling and user feedback
pub struct GuiApp {
    /// The emulator system being monitored and controlled
    /// Wrapped in Arc<Mutex<>> for thread-safe access
    system: Option<Arc<Mutex<ConfigurableSystem>>>,
    /// GUI-specific state management
    gui_state: state::GuiState,
    /// Container for all GUI components
    components: components::GuiComponents,
}

impl GuiApp {
    /// Create a new GUI application instance
    ///
    /// This constructor initializes all GUI components and state management systems.
    /// The application starts without a system loaded - use `set_system()` to connect
    /// an emulator instance.
    ///
    /// # Arguments
    /// * `_cc` - eframe creation context (currently unused but required by the framework)
    ///
    /// # Returns
    /// A new `GuiApp` instance ready for system integration
    ///
    /// # Example
    /// ```rust,no_run
    /// use rusty_emu::gui::GuiApp;
    ///
    /// let app = GuiApp::new(&eframe::CreationContext {
    ///     egui_ctx: Default::default(),
    ///     integration_info: Default::default(),
    ///     storage: None,
    /// });
    /// ```
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let gui_state = state::GuiState::new();
        let components = components::GuiComponents::new();

        Self {
            system: None,
            gui_state,
            components,
        }
    }

    /// Set the emulator system for the GUI to control and monitor
    ///
    /// This method establishes the connection between the GUI and an emulator system.
    /// Once connected, the GUI will begin monitoring system state and providing
    /// interactive controls.
    ///
    /// # Arguments
    /// * `system` - Thread-safe reference to the emulator system
    ///
    /// # Thread Safety
    /// This method is not thread-safe and should only be called during GUI initialization
    /// or when no emulation is running.
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::sync::{Arc, Mutex};
    /// use rusty_emu::gui::GuiApp;
    /// use rusty_emu::system_config::{ConfigurableSystem, SystemFactory};
    ///
    /// let mut app = GuiApp::new(&creation_context);
    /// let factory = SystemFactory::new();
    /// let system = Arc::new(Mutex::new(factory.create_from_json("config.json")?));
    /// app.set_system(system);
    /// ```
    pub fn set_system(&mut self, system: Arc<Mutex<ConfigurableSystem>>) {
        self.system = Some(system);
        self.gui_state.system_loaded = true;
    }

    /// Get current system reference if available
    ///
    /// Returns a cloned Arc reference to the system for thread-safe access.
    /// This method is used internally by the GUI to update state and render
    /// system information.
    ///
    /// # Returns
    /// `Some(Arc<Mutex<ConfigurableSystem>>)` if a system is loaded, `None` otherwise
    fn get_system(&self) -> Option<Arc<Mutex<ConfigurableSystem>>> {
        self.system.as_ref().cloned()
    }
}

impl eframe::App for GuiApp {
    /// Main update loop for the GUI application
    ///
    /// This method is called by eframe for each frame and handles:
    /// - Requesting continuous repaints for real-time updates
    /// - Updating GUI state from the emulator system
    /// - Rendering the complete user interface
    ///
    /// # Arguments
    /// * `ctx` - egui context for rendering and interaction
    /// * `_frame` - eframe frame (currently unused)
    ///
    /// # Performance
    /// - Requests repaint at ~60 FPS for smooth real-time interaction
    /// - State updates are performed without blocking the GUI thread
    /// - System lock is held briefly to copy current state
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint for smooth real-time updates (60 FPS)
        ctx.request_repaint();

        // Update system state if available - non-blocking operation
        if let Some(system) = self.get_system() {
            self.gui_state.update_from_system(&system);
        }

        // Render the complete GUI interface
        self.render_gui(ctx);
    }
}

impl GuiApp {
    /// Render the complete GUI interface
    ///
    /// This method orchestrates the rendering of all GUI components in their
    /// proper layout. It creates a clean, organized interface with logical
    /// grouping of related functionality.
    ///
    /// # Arguments
    /// * `ctx` - egui context for rendering operations
    ///
    /// # Layout Structure
    /// - Header with application title
    /// - Control panel for system management
    /// - Memory viewer for RAM inspection
    /// - Register viewer for CPU state
    /// - Status bar for system health and errors
    fn render_gui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Application header
            ui.heading("Intel MCS-4 Emulator");

            ui.separator();

            // Main GUI sections - organized for optimal workflow
            self.components
                .render_control_panel(ui, &mut self.gui_state);
            self.components.render_memory_viewer(ui, &self.gui_state);
            self.components.render_register_viewer(ui, &self.gui_state);
            self.components.render_status_bar(ui, &self.gui_state);
        });
    }
}

/// Run the GUI application as a standalone desktop application
///
/// This function launches the complete GUI application with the specified system.
/// It handles all eframe initialization, window creation, and event loop management.
///
/// # Arguments
/// * `system` - Optional system to load into the GUI (can be None for manual loading)
///
/// # Returns
/// `eframe::Result<()>` - Success or failure of GUI initialization and execution
///
/// # Errors
/// Returns an error if:
/// - eframe initialization fails
/// - Window creation fails
/// - GUI event loop encounters critical errors
/// - Display server is not available (headless environment)
///
/// # Example
/// ```rust,no_run
/// use std::sync::{Arc, Mutex};
/// use rusty_emu::gui::run_gui;
/// use rusty_emu::system_config::{ConfigurableSystem, SystemFactory};
///
/// // Create system (optional)
/// let factory = SystemFactory::new();
/// let system = Some(Arc::new(Mutex::new(
///     factory.create_from_json("configs/mcs4_basic.json")?
/// )));
///
/// // Launch GUI - this will block until the GUI window is closed
/// if let Err(e) = run_gui(system) {
///     eprintln!("GUI failed to start: {}", e);
/// }
/// ```
///
/// # Window Configuration
/// - **Default Size**: 1200x800 pixels
/// - **Title**: "Intel MCS-4 Emulator"
/// - **Resizable**: Yes (default eframe behavior)
/// - **Frame Rate**: ~60 FPS for smooth interaction
///
/// # Platform Notes
/// - **Windows**: Requires Windows 10 or later
/// - **Linux**: Requires X11 or Wayland display server
/// - **macOS**: Requires macOS 10.15 or later
pub fn run_gui(system: Option<Arc<Mutex<ConfigurableSystem>>>) -> eframe::Result<()> {
    // Configure native window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Intel MCS-4 Emulator"),
        ..Default::default()
    };

    // Launch the native GUI application
    // eframe will handle CreationContext creation internally
    eframe::run_native(
        "Intel MCS-4 Emulator",
        options,
        Box::new(|cc| {
            let mut app = GuiApp::new(cc);

            // Connect system if provided
            if let Some(sys) = system {
                app.set_system(sys);
            }

            Box::new(app)
        }),
    )
}
