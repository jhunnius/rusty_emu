use std::sync::Arc;
use crate::pin::Pin;

pub fn connect_pins(source: &Arc<Pin>, destination: &Arc<Pin>) -> Result<(), String> {
    let mut source_connections = source.connections.lock().unwrap();
    source_connections.push(destination.clone());

    let mut dest_connections = destination.connections.lock().unwrap();
    dest_connections.push(source.clone());

    Ok(())
}