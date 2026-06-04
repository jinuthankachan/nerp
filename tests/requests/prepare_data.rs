//! Shared test helpers for HTTP request tests.
//!
//! Many auth tests need a user who is already registered, verified, and logged
//! in. Rather than repeat that setup, they call `init_user_login` here. This
//! module also builds the `Authorization` header used to call protected routes.

use axum::http::{HeaderName, HeaderValue};
use loco_rs::{app::AppContext, TestServer};
use nerp::{models::users, views::auth::LoginResponse};

// Fixed credentials reused across tests so results are predictable.
const USER_EMAIL: &str = "test@loco.com";
const USER_PASSWORD: &str = "1234";

/// Bundles the things a test needs after logging a user in: the user record and
/// the auth token to send on subsequent requests.
pub struct LoggedInUser {
    pub user: users::Model,
    pub token: String,
}

/// Register a fresh user, verify their email, log them in, and return both the
/// user row and their auth token. `request` is the in-memory test HTTP client;
/// `ctx` gives direct database access to read the user back.
pub async fn init_user_login(request: &TestServer, ctx: &AppContext) -> LoggedInUser {
    let register_payload = serde_json::json!({
        "name": "loco",
        "email": USER_EMAIL,
        "password": USER_PASSWORD
    });

    //Creating a new user
    request
        .post("/api/auth/register")
        .json(&register_payload)
        .await;
    // Read the just-created user straight from the DB to get their tokens.
    let user = users::Model::find_by_email(&ctx.db, USER_EMAIL)
        .await
        .unwrap();

    let verify_payload = serde_json::json!({
        "token": user.email_verification_token,
    });

    // Confirm the email so the account can log in.
    request.post("/api/auth/verify").json(&verify_payload).await;

    // Log in and capture the response (which contains the auth token).
    let response = request
        .post("/api/auth/login")
        .json(&serde_json::json!({
            "email": USER_EMAIL,
            "password": USER_PASSWORD
        }))
        .await;

    // Parse the JSON body into our typed `LoginResponse` to read the token.
    let login_response: LoginResponse = serde_json::from_str(&response.text()).unwrap();

    LoggedInUser {
        // Re-fetch the user so the returned record reflects the verified state.
        user: users::Model::find_by_email(&ctx.db, USER_EMAIL)
            .await
            .unwrap(),
        token: login_response.token,
    }
}

/// Build the `Authorization: Bearer <token>` header pair that protected
/// endpoints (like `/api/auth/current`) require.
pub fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let auth_header_value = HeaderValue::from_str(&format!("Bearer {}", &token)).unwrap();

    (HeaderName::from_static("authorization"), auth_header_value)
}
