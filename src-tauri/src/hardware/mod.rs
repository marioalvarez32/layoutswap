//! The Windows hardware adapter: `probe() -> Inventory`.
//!
//! The implementation runs the probe script through a [`crate::script::run::ScriptRunner`]
//! and parses its JSON into monitors and audio endpoints. Hardware knowledge lives in this
//! one directory; nothing else in the crate reads a device path or a GPU identity.
//!
//! The probe script, the `Monitor` type and `probe` itself arrive with the capture ticket.
