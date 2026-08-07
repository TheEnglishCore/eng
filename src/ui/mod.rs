#[cfg(feature = "ui")]
mod bridge;

#[cfg(feature = "ui")]
pub use bridge::*;

#[cfg(not(feature = "ui"))]
use crate::error::{EnglingError, Result};

#[cfg(not(feature = "ui"))]
pub fn run_event_loop() -> Result<()> {
    Err(EnglingError::runtime(
        "UI support is not enabled. Rebuild with: cargo build --features ui",
    ))
}

#[cfg(not(feature = "ui"))]
pub fn init_ui() {}

#[cfg(not(feature = "ui"))]
pub fn register_vm(_vm: &crate::vm::VM) {}

#[cfg(not(feature = "ui"))]
pub fn process_ui_statement(_stmt: &crate::ast::Statement) -> crate::error::Result<()> {
    Ok(())
}

#[cfg(not(feature = "ui"))]
pub fn update_label_text(_name: &str, _text: String) -> crate::error::Result<()> {
    Ok(())
}
