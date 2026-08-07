//! M4 workflow engine (ADR-0012): a lightweight "n8n-style" workflow runner
//! that stays independent of Tauri. Model + executor live here; the app layer
//! (crate::workflow) provides persistence, agent calls and window/event sinks.

pub mod engine;
pub mod model;