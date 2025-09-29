use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::component::Component;
use std::sync::{Arc, Mutex};

/// JSON-based system configuration structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub name: String,
    pub description: String,
    pub version: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub components: HashMap<String, ComponentConfig>,
    pub connections: HashMap<String, ConnectionConfig>,
    pub layout: Option<LayoutConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentConfig {
    #[serde(rename = "single")]
    Single(SingleComponentConfig),
    #[serde(rename = "array")]
    Array(ArrayComponentConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleComponentConfig {
    pub component_type: String,
    pub name: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrayComponentConfig {
    pub component_type: String,
    pub count: usize,
    pub naming_pattern: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub overrides: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub connection_type: String,
    pub source: PinReference,
    pub targets: Vec<PinReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinReference {
    pub component: String,
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub grid_size: [usize; 2],
    pub positions: HashMap<String, [usize; 2]>,
}

/// System factory for creating systems from JSON configuration
pub struct SystemFactory {
    component_registry: HashMap<String, fn(config: &ComponentConfig, name: String) -> Result<Box<dyn Component>, String>>,
}

impl SystemFactory {
    pub fn new() -> Self {
        let mut factory = SystemFactory {
            component_registry: HashMap::new(),
        };
        factory.register_default_components();
        factory
    }

    fn register_default_components(&mut self) {
        // Register component creation functions
        self.component_registry.insert(
            "intel_4004".to_string(),
            |config: &ComponentConfig, name: String| {
                if let ComponentConfig::Single(single) = config {
                    let clock_speed = single.properties.get("clock_speed")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(750000.0);
                    Ok(Box::new(crate::components::cpu::intel_4004::Intel4004::new(name, clock_speed)))
                } else {
                    Err("Intel 4004 must be single component".to_string())
                }
            }
        );

        self.component_registry.insert(
            "generic_clock".to_string(),
            |config: &ComponentConfig, name: String| {
                if let ComponentConfig::Single(single) = config {
                    let frequency = single.properties.get("frequency")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(750000.0);
                    Ok(Box::new(crate::components::clock::generic_clock::GenericClock::new(name, frequency)))
                } else {
                    Err("Generic clock must be single component".to_string())
                }
            }
        );

        self.component_registry.insert(
            "intel_4001".to_string(),
            |config: &ComponentConfig, name: String| {
                if let ComponentConfig::Single(_single) = config {
                    Ok(Box::new(crate::components::memory::intel_4001::Intel4001::new(name)))
                } else {
                    Err("Intel 4001 must be single component".to_string())
                }
            }
        );

        self.component_registry.insert(
            "intel_4002".to_string(),
            |config: &ComponentConfig, name: String| {
                if let ComponentConfig::Single(single) = config {
                    let variant = single.properties.get("variant")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Type1");
                    let access_time = single.properties.get("access_time")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(500);

                    let ram_variant = match variant {
                        "Type2" => crate::components::memory::intel_4002::RamVariant::Type2,
                        _ => crate::components::memory::intel_4002::RamVariant::Type1,
                    };

                    Ok(Box::new(crate::components::memory::intel_4002::Intel4002::new_with_variant_and_access_time(
                        name, ram_variant, access_time
                    )))
                } else {
                    Err("Intel 4002 must be single component".to_string())
                }
            }
        );

        self.component_registry.insert(
            "intel_4003".to_string(),
            |config: &ComponentConfig, name: String| {
                if let ComponentConfig::Single(_single) = config {
                    Ok(Box::new(crate::components::memory::intel_4003::Intel4003::new(name)))
                } else {
                    Err("Intel 4003 must be single component".to_string())
                }
            }
        );
    }

    pub fn create_from_json(&self, json_path: &str) -> Result<ConfigurableSystem, String> {
        let config: SystemConfig = self.load_json_config(json_path)?;
        let mut components = self.create_components(&config)?;
        self.connect_components(&config, &mut components)?;
        Ok(ConfigurableSystem::new(config, components))
    }

    fn load_json_config(&self, path: &str) -> Result<SystemConfig, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON config '{}': {}", path, e))
    }

    fn create_components(&self, config: &SystemConfig) -> Result<HashMap<String, Arc<Mutex<Box<dyn Component>>>>, String> {
        let mut components = HashMap::new();

        for (id, component_config) in &config.components {
            let component_names = self.expand_component_names(id, component_config);

            for component_name in component_names {
                let component = self.create_single_component(component_config, component_name.clone())?;
                components.insert(component_name, Arc::new(Mutex::new(component)));
            }
        }

        Ok(components)
    }

    fn expand_component_names(&self, _id: &str, config: &ComponentConfig) -> Vec<String> {
        match config {
            ComponentConfig::Single(single) => vec![single.name.clone()],
            ComponentConfig::Array(array) => {
                let mut names = Vec::new();
                for i in 0..array.count {
                    let name = array.naming_pattern.replace("{:02}", &format!("{:02}", i));
                    names.push(name);
                }
                names
            }
        }
    }

    fn create_single_component(&self, config: &ComponentConfig, name: String) -> Result<Box<dyn Component>, String> {
        match config {
            ComponentConfig::Single(single) => {
                if let Some(creator) = self.component_registry.get(&single.component_type) {
                    creator(config, name)
                } else {
                    Err(format!("Unknown component type: {}", single.component_type))
                }
            }
            ComponentConfig::Array(array) => {
                if let Some(creator) = self.component_registry.get(&array.component_type) {
                    creator(config, name)
                } else {
                    Err(format!("Unknown component type: {}", array.component_type))
                }
            }
        }
    }

    fn connect_components(&self, config: &SystemConfig, components: &mut HashMap<String, Arc<Mutex<Box<dyn Component>>>>) -> Result<(), String> {
        for (connection_id, connection_config) in &config.connections {
            println!("Connecting: {}", connection_id);

            // Get source pin
            let source_component = components.get(&connection_config.source.component)
                .ok_or_else(|| format!("Source component not found: {}", connection_config.source.component))?;

            let source_pin = {
                let component = source_component.lock().unwrap();
                component.get_pin(&connection_config.source.pin)
                    .map_err(|e| format!("Failed to get source pin: {}", e))?
            };

            // Connect to all targets
            for target_ref in &connection_config.targets {
                let target_component = components.get(&target_ref.component)
                    .ok_or_else(|| format!("Target component not found: {}", target_ref.component))?;

                let target_pin = {
                    let component = target_component.lock().unwrap();
                    component.get_pin(&target_ref.pin)
                        .map_err(|e| format!("Failed to get target pin: {}", e))?
                };

                // Connect the pins
                let _source_pin_guard = source_pin.lock().unwrap();
                let mut target_pin_guard = target_pin.lock().unwrap();
                target_pin_guard.connect_to(source_pin.clone());
            }
        }

        Ok(())
    }
}

/// A configurable system created from JSON configuration
pub struct ConfigurableSystem {
    config: SystemConfig,
    components: HashMap<String, Arc<Mutex<Box<dyn Component>>>>,
    is_running: bool,
}

impl ConfigurableSystem {
    pub fn new(config: SystemConfig, components: HashMap<String, Arc<Mutex<Box<dyn Component>>>>) -> Self {
        ConfigurableSystem {
            config,
            components,
            is_running: false,
        }
    }

    pub fn run(&mut self) {
        self.is_running = true;
        let mut handles = vec![];

        println!("Starting configurable system: {}", self.config.name);
        println!("Description: {}", self.config.description);

        for (name, component) in &self.components {
            let comp_clone = Arc::clone(component);
            let name_clone = name.clone();

            let handle = std::thread::spawn(move || {
                println!("Starting component: {}", name_clone);
                if let Ok(mut comp) = comp_clone.lock() {
                    comp.run();
                }
                println!("Component {} stopped", name_clone);
            });

            handles.push((name.clone(), handle));
        }

        println!("All components started. System running...");
        std::thread::sleep(std::time::Duration::from_secs(2));
        self.is_running = false;

        // Stop all components
        println!("\nStopping system components...");
        for (name, component) in &self.components {
            if let Ok(mut comp) = component.lock() {
                comp.stop();
                println!("Stopped component: {}", name);
            }
        }

        // Wait for threads
        for (name, handle) in handles {
            match handle.join() {
                Ok(_) => println!("Component {} thread finished", name),
                Err(_) => eprintln!("Component {} thread panicked", name),
            }
        }

        println!("Configurable system stopped.");
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn get_system_info(&self) -> SystemInfo {
        let rom_size = self.config.metadata.get("rom_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(256) as usize;
        let ram_size = self.config.metadata.get("ram_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(40) as usize;

        SystemInfo {
            name: self.config.name.clone(),
            description: self.config.description.clone(),
            component_count: self.components.len(),
            cpu_speed: self.config.metadata.get("cpu_speed")
                .and_then(|v| v.as_f64())
                .unwrap_or(750000.0),
            rom_size,
            ram_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub name: String,
    pub description: String,
    pub component_count: usize,
    pub cpu_speed: f64,
    pub rom_size: usize,
    pub ram_size: usize,
}