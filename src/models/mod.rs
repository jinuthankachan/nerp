//! Models module — the data layer.
//!
//! `_entities` holds the auto-generated table definitions; `users` adds the
//! hand-written behavior on top of them.

pub mod _entities; // generated table structs (don't hand-edit these)
pub mod users; // user business logic (validation, lookups, tokens, …)
