//! Auth views — the *shapes* of the JSON the auth endpoints send back.
//!
//! A "view" here is just a plain struct describing a response. Keeping these
//! separate from the database `Model` lets us expose only the safe, relevant
//! fields (e.g. never the password hash) and shape the JSON how clients expect.

use serde::{Deserialize, Serialize};

use crate::models::_entities::users;

/// What `POST /login` (and magic-link login) returns: the auth token plus a few
/// harmless profile details, so the client doesn't need a second request.
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResponse {
    pub token: String, // the JWT the client stores and sends on later requests
    pub pid: String,   // the user's public id
    pub name: String,
    pub is_verified: bool, // whether the email has been confirmed
}

impl LoginResponse {
    // Build a `LoginResponse` from a fetched user and a freshly-minted token.
    // `#[must_use]` warns if the caller forgets to use the value this returns.
    #[must_use]
    pub fn new(user: &users::Model, token: &String) -> Self {
        Self {
            token: token.to_string(),
            pid: user.pid.to_string(),
            name: user.name.clone(),
            // `is_some()` is true when a verification timestamp exists.
            is_verified: user.email_verified_at.is_some(),
        }
    }
}

/// What `GET /current` returns: the logged-in user's basic profile.
#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentResponse {
    pub pid: String,
    pub name: String,
    pub email: String,
}

impl CurrentResponse {
    // Copy just the public-safe fields out of the full user record.
    #[must_use]
    pub fn new(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}
