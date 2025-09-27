use crate::pin::Pin;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Manages electrical connections between pins
pub struct ConnectionManager {
    pin_registry: HashMap<String, Arc<Mutex<Pin>>>,
    connections: HashMap<String, Vec<String>>, // pin_name -> connected_pin_names
}

impl ConnectionManager {
    pub fn new() -> Self {
        ConnectionManager {
            pin_registry: HashMap::new(),
            connections: HashMap::new(),
        }
    }

    pub fn register_pin(&mut self, name: String, pin: Arc<Mutex<Pin>>) {
        self.pin_registry.insert(name, pin);
    }

    pub fn get_pin(&self, name: &str) -> Option<Arc<Mutex<Pin>>> {
        self.pin_registry.get(name).cloned()
    }

    /// Connect two pins bidirectionally
    pub fn connect_pins(
        &mut self,
        pin1: Arc<Mutex<Pin>>,
        pin2: Arc<Mutex<Pin>>,
    ) -> Result<(), String> {
        let pin1_name = {
            let p1 = pin1
                .lock()
                .map_err(|e| format!("Failed to lock pin1: {}", e))?;
            p1.name().to_string()
        };

        let pin2_name = {
            let p2 = pin2
                .lock()
                .map_err(|e| format!("Failed to lock pin2: {}", e))?;
            p2.name().to_string()
        };

        // Connect pin1 to pin2
        {
            let mut p1 = pin1
                .lock()
                .map_err(|e| format!("Failed to lock pin1: {}", e))?;
            p1.connect_to(pin2.clone());
        }

        // Connect pin2 to pin1 (bidirectional)
        {
            let mut p2 = pin2
                .lock()
                .map_err(|e| format!("Failed to lock pin2: {}", e))?;
            p2.connect_to(pin1.clone());
        }

        // Update connection graph
        self.connections
            .entry(pin1_name.clone())
            .or_insert_with(Vec::new)
            .push(pin2_name.clone());

        self.connections
            .entry(pin2_name)
            .or_insert_with(Vec::new)
            .push(pin1_name);

        Ok(())
    }

    /// Connect multiple pins together (bus connection)
    pub fn connect_bus(&mut self, pins: &[Arc<Mutex<Pin>>]) -> Result<(), String> {
        if pins.len() < 2 {
            return Err("Need at least 2 pins for bus connection".to_string());
        }

        for i in 0..pins.len() {
            for j in i + 1..pins.len() {
                self.connect_pins(pins[i].clone(), pins[j].clone())?;
            }
        }

        Ok(())
    }

    /// Disconnect two pins
    pub fn disconnect_pins(
        &mut self,
        pin1: &Arc<Mutex<Pin>>,
        pin2: &Arc<Mutex<Pin>>,
    ) -> Result<(), String> {
        let pin1_name = {
            let p1 = pin1
                .lock()
                .map_err(|e| format!("Failed to lock pin1: {}", e))?;
            p1.name().to_string()
        };

        let pin2_name = {
            let p2 = pin2
                .lock()
                .map_err(|e| format!("Failed to lock pin2: {}", e))?;
            p2.name().to_string()
        };

        // Disconnect pin1 from pin2
        {
            let mut p1 = pin1
                .lock()
                .map_err(|e| format!("Failed to lock pin1: {}", e))?;
            p1.disconnect_from_pin(pin2);
        }

        // Disconnect pin2 from pin1
        {
            let mut p2 = pin2
                .lock()
                .map_err(|e| format!("Failed to lock pin2: {}", e))?;
            p2.disconnect_from_pin(pin1);
        }

        // Update connection graph
        if let Some(connections) = self.connections.get_mut(&pin1_name) {
            connections.retain(|name| name != &pin2_name);
        }

        if let Some(connections) = self.connections.get_mut(&pin2_name) {
            connections.retain(|name| name != &pin1_name);
        }

        Ok(())
    }

    /// Get all pins connected to a given pin
    pub fn get_connected_pins(&self, pin_name: &str) -> Option<&Vec<String>> {
        self.connections.get(pin_name)
    }

    /// Check if two pins are connected
    pub fn are_connected(&self, pin1_name: &str, pin2_name: &str) -> bool {
        self.connections
            .get(pin1_name)
            .map(|connections| connections.contains(&pin2_name.to_string()))
            .unwrap_or(false)
    }

    /// Disconnect all pins from a given pin
    pub fn disconnect_all(&mut self, pin: &Arc<Mutex<Pin>>) -> Result<(), String> {
        let pin_name = {
            let p = pin
                .lock()
                .map_err(|e| format!("Failed to lock pin: {}", e))?;
            p.name().to_string()
        };

        // Get all connected pin names before disconnecting
        let connected_names: Vec<String> = self
            .connections
            .get(&pin_name)
            .map(|v| v.clone())
            .unwrap_or_default();

        // Disconnect from each connected pin
        for connected_name in &connected_names {
            // We need to find the actual Pin objects to disconnect them
            // For now, we'll handle this through the connection graph
            if let Some(connections) = self.connections.get_mut(connected_name) {
                connections.retain(|name| name != &pin_name);
            }
        }

        // Clear all connections for this pin
        if let Some(connections) = self.connections.get_mut(&pin_name) {
            connections.clear();
        }

        // Clear the pin's internal connections
        {
            let mut p = pin
                .lock()
                .map_err(|e| format!("Failed to lock pin: {}", e))?;
            p.clear_connections();
        }

        Ok(())
    }

    /// Get a list of all connection groups (useful for debugging)
    pub fn get_connection_groups(&self) -> Vec<Vec<String>> {
        use std::collections::{HashSet, VecDeque};

        let mut visited = HashSet::new();
        let mut groups = Vec::new();

        for pin_name in self.connections.keys() {
            if !visited.contains(pin_name) {
                let mut group = Vec::new();
                let mut queue = VecDeque::new();
                queue.push_back(pin_name.clone());

                while let Some(current) = queue.pop_front() {
                    if visited.insert(current.clone()) {
                        group.push(current.clone());

                        if let Some(neighbors) = self.connections.get(&current) {
                            for neighbor in neighbors {
                                if !visited.contains(neighbor) {
                                    queue.push_back(neighbor.clone());
                                }
                            }
                        }
                    }
                }

                if group.len() > 1 {
                    groups.push(group);
                }
            }
        }

        groups
    }
}

/// Helper function for quick pin connections
pub fn connect_pins(pin1: Arc<Mutex<Pin>>, pin2: Arc<Mutex<Pin>>) -> Result<(), String> {
    let mut manager = ConnectionManager::new();
    manager.connect_pins(pin1, pin2)
}

/// Helper function for bus connections
pub fn connect_bus(pins: &[Arc<Mutex<Pin>>]) -> Result<(), String> {
    let mut manager = ConnectionManager::new();
    manager.connect_bus(pins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_connection() {
        let pin1 = Arc::new(Mutex::new(Pin::new("PIN1".to_string())));
        let pin2 = Arc::new(Mutex::new(Pin::new("PIN2".to_string())));

        assert!(connect_pins(pin1.clone(), pin2.clone()).is_ok());

        // Verify connection
        let p1 = pin1.lock().unwrap();
        let p2 = pin2.lock().unwrap();

        assert_eq!(p1.get_connection_count(), 1);
        assert_eq!(p2.get_connection_count(), 1);
        assert!(p1.is_connected_to(&pin2));
        assert!(p2.is_connected_to(&pin1));
    }

    #[test]
    fn test_pin_disconnection() {
        let pin1 = Arc::new(Mutex::new(Pin::new("PIN1".to_string())));
        let pin2 = Arc::new(Mutex::new(Pin::new("PIN2".to_string())));

        let mut manager = ConnectionManager::new();
        manager.connect_pins(pin1.clone(), pin2.clone()).unwrap();

        manager.disconnect_pins(&pin1, &pin2).unwrap();

        let p1 = pin1.lock().unwrap();
        let p2 = pin2.lock().unwrap();

        assert_eq!(p1.get_connection_count(), 0);
        assert_eq!(p2.get_connection_count(), 0);
        assert!(!p1.is_connected_to(&pin2));
        assert!(!p2.is_connected_to(&pin1));
    }

    #[test]
    fn test_bus_connection() {
        let pins: Vec<_> = (0..4)
            .map(|i| Arc::new(Mutex::new(Pin::new(format!("PIN{}", i)))))
            .collect();

        assert!(connect_bus(&pins).is_ok());

        // Verify all pins are connected to each other
        for i in 0..pins.len() {
            let pin = pins[i].lock().unwrap();
            assert_eq!(pin.get_connection_count(), pins.len() - 1);
        }
    }

    #[test]
    fn test_disconnect_all() {
        let pin1 = Arc::new(Mutex::new(Pin::new("PIN1".to_string())));
        let pin2 = Arc::new(Mutex::new(Pin::new("PIN2".to_string())));
        let pin3 = Arc::new(Mutex::new(Pin::new("PIN3".to_string())));

        let mut manager = ConnectionManager::new();
        manager.connect_pins(pin1.clone(), pin2.clone()).unwrap();
        manager.connect_pins(pin1.clone(), pin3.clone()).unwrap();

        manager.disconnect_all(&pin1).unwrap();

        let p1 = pin1.lock().unwrap();
        assert_eq!(p1.get_connection_count(), 0);
    }
}
