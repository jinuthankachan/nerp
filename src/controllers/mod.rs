//! Controllers module — index of all request handlers.

pub mod auth; // JSON API: register/login/verify/reset/magic-link/current
pub mod home; // server-rendered HTML pages: `/` and `/htmx-probe`
