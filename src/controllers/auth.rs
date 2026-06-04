//! Authentication controller — the JSON API for everything account-related:
//! registering, verifying an email, logging in, password reset, magic links,
//! and fetching the current user.
//!
//! Each `async fn` below is one HTTP endpoint. They follow loco's convention of
//! staying "thin": they read the request, call a method on the user *model*
//! (where the real logic lives, in src/models/users.rs), and return JSON. The
//! route table at the very bottom connects each function to its URL.

use crate::{
    mailers::auth::AuthMailer, // sends welcome / reset / magic-link emails
    models::{
        _entities::users,                     // the raw `users` database table type
        users::{LoginParams, RegisterParams}, // request-body shapes for login/register
    },
    views::auth::{CurrentResponse, LoginResponse}, // shapes of the JSON we return
};
use loco_rs::prelude::*; // Response, State, Json, get/post, error helpers, …
use regex::Regex; // regular expressions (used to validate email domains)
use serde::{Deserialize, Serialize}; // turn structs into/out of JSON
use std::sync::OnceLock; // a "set exactly once" container, used for the regex

// `OnceLock` lets us build the email regex a single time and reuse it. Compiling
// a regex isn't free, so we cache it instead of rebuilding it on every request.
pub static EMAIL_DOMAIN_RE: OnceLock<Regex> = OnceLock::new();

// Return the cached regex, compiling it on first use. `get_or_init` runs the
// closure only the first time; afterwards it hands back the stored value.
fn get_allow_email_domain_re() -> &'static Regex {
    EMAIL_DOMAIN_RE.get_or_init(|| {
        // Only emails ending in @example.com or @gmail.com are allowed to
        // request a magic link. `expect` crashes loudly if this hand-written
        // pattern is invalid — that would be a programming bug, not user input.
        Regex::new(r"@example\.com$|@gmail\.com$").expect("Failed to compile regex")
    })
}

// The structs below describe the JSON bodies clients send to certain endpoints.
// `#[derive(Deserialize, Serialize)]` auto-generates the code to convert them
// to and from JSON, so we never parse JSON by hand.

/// Body of `POST /forgot` — the email asking for a password-reset link.
#[derive(Debug, Deserialize, Serialize)]
pub struct ForgotParams {
    pub email: String,
}

/// Body of `POST /reset` — the reset token plus the new password to set.
#[derive(Debug, Deserialize, Serialize)]
pub struct ResetParams {
    pub token: String,
    pub password: String,
}

/// Body of `POST /magic-link` — the email to send a passwordless login link to.
#[derive(Debug, Deserialize, Serialize)]
pub struct MagicLinkParams {
    pub email: String,
}

/// Body of `POST /resend-verification-mail` — the email to re-send verification to.
#[derive(Debug, Deserialize, Serialize)]
pub struct ResendVerificationParams {
    pub email: String,
}

/// Register function creates a new user with the given parameters and sends a
/// welcome email to the user.
///
/// `State(ctx)` gives us the shared app context (database, config, mailer).
/// `Json(params)` is the request body, already parsed into a `RegisterParams`.
/// `#[debug_handler]` just improves compiler error messages for handlers.
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    // Try to create the user (hashes the password, inserts the row).
    let res = users::Model::create_with_password(&ctx.db, &params).await;

    // `match` inspects the result. On success we keep the user; on failure we
    // log it and *still* return 200 OK with an empty body — we deliberately
    // don't reveal whether registration failed (e.g. email already taken).
    let user = match res {
        Ok(user) => user,
        Err(err) => {
            tracing::info!(
                message = err.to_string(),
                user_email = &params.email,
                "could not register user",
            );
            return format::json(());
        }
    };

    // Generate and store a verification token + timestamp on the new user.
    // `into_active_model()` converts the saved row into an editable form.
    // `?` means: if this errors, stop and return the error from this handler.
    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    // Email the user their welcome + verification link.
    AuthMailer::send_welcome(&ctx, &user).await?;

    // Respond 200 OK with an empty JSON body.
    format::json(())
}

/// Verify register user. if the user not verified his email, he can't login to
/// the system.
///
/// `Path(token)` pulls the `{token}` segment out of the URL.
#[debug_handler]
async fn verify(State(ctx): State<AppContext>, Path(token): Path<String>) -> Result<Response> {
    // `let Ok(user) = ... else { ... }` is a concise way to say: if the lookup
    // succeeds, bind `user`; otherwise run the `else` block. Here a bad token
    // means we reject the request as unauthorized.
    let Ok(user) = users::Model::find_by_verification_token(&ctx.db, &token).await else {
        return unauthorized("invalid token");
    };

    // `email_verified_at` is an `Option` (a value that may be absent).
    // `.is_some()` is true when it already holds a timestamp — i.e. already
    // verified, so there's nothing to do but log it.
    if user.email_verified_at.is_some() {
        tracing::info!(pid = user.pid.to_string(), "user already verified");
    } else {
        // Otherwise mark the email verified now and log the change.
        let active_model = user.into_active_model();
        let user = active_model.verified(&ctx.db).await?;
        tracing::info!(pid = user.pid.to_string(), "user verified");
    }

    format::json(())
}

/// In case the user forgot his password  this endpoints generate a forgot token
/// and send email to the user. In case the email not found in our DB, we are
/// returning a valid request for for security reasons (not exposing users DB
/// list).
#[debug_handler]
async fn forgot(
    State(ctx): State<AppContext>,
    Json(params): Json<ForgotParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return format::json(());
    };

    // Generate + store a reset token and timestamp on the user.
    let user = user
        .into_active_model()
        .set_forgot_password_sent(&ctx.db)
        .await?;

    // Email the reset link to the user.
    AuthMailer::forgot_password(&ctx, &user).await?;

    format::json(())
}

/// reset user password by the given parameters
#[debug_handler]
async fn reset(State(ctx): State<AppContext>, Json(params): Json<ResetParams>) -> Result<Response> {
    let Ok(user) = users::Model::find_by_reset_token(&ctx.db, &params.token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::info!("reset token not found");

        return format::json(());
    };
    // Hash the new password, store it, and clear the now-used reset token.
    user.into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await?;

    format::json(())
}

/// Creates a user login and returns a token
#[debug_handler]
async fn login(State(ctx): State<AppContext>, Json(params): Json<LoginParams>) -> Result<Response> {
    // Find the user by email. Unknown email → 401 (but we log only at debug
    // level and give the same generic message as a wrong password, so attackers
    // can't tell which emails exist).
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        tracing::debug!(
            email = params.email,
            "login attempt with non-existent email"
        );
        return unauthorized("Invalid credentials!");
    };

    // Compare the submitted password against the stored hash.
    let valid = user.verify_password(&params.password);

    // Wrong password → also 401.
    if !valid {
        return unauthorized("unauthorized!");
    }

    // Read the JWT signing secret/expiry from config.
    let jwt_secret = ctx.config.get_jwt_config()?;

    // Mint a signed token proving who this user is. If signing fails for any
    // reason, fall back to a 401 rather than leaking details.
    let token = user
        .generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    // Return the token plus a little user info as JSON.
    format::json(LoginResponse::new(&user, &token))
}

/// Return the currently-logged-in user's details.
///
/// `auth: auth::JWT` makes loco require and validate a JWT on this request;
/// if the token is missing or invalid, the handler is never even reached.
#[debug_handler]
async fn current(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    // `auth.claims.pid` is the user id baked into the token. Look that user up.
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    format::json(CurrentResponse::new(&user))
}

/// Magic link authentication provides a secure and passwordless way to log in to the application.
///
/// # Flow
/// 1. **Request a Magic Link**:
///    A registered user sends a POST request to `/magic-link` with their email.
///    If the email exists, a short-lived, one-time-use token is generated and sent to the user's email.
///    For security and to avoid exposing whether an email exists, the response always returns 200, even if the email is invalid.
///
/// 2. **Click the Magic Link**:
///    The user clicks the link (/magic-link/{token}), which validates the token and its expiration.
///    If valid, the server generates a JWT and responds with a [`LoginResponse`].
///    If invalid or expired, an unauthorized response is returned.
///
/// This flow enhances security by avoiding traditional passwords and providing a seamless login experience.
async fn magic_link(
    State(ctx): State<AppContext>,
    Json(params): Json<MagicLinkParams>,
) -> Result<Response> {
    // First gate: the email must match an allowed domain (see the regex above).
    // A non-matching email is rejected outright with 400 Bad Request.
    let email_regex = get_allow_email_domain_re();
    if !email_regex.is_match(&params.email) {
        tracing::debug!(
            email = params.email,
            "The provided email is invalid or does not match the allowed domains"
        );
        return bad_request("invalid request");
    }

    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::debug!(email = params.email, "user not found by email");
        return format::empty_json();
    };

    // Generate + store a short-lived magic-link token, then email it.
    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    AuthMailer::send_magic_link(&ctx, &user).await?;

    format::empty_json()
}

/// Verifies a magic link token and authenticates the user.
async fn magic_link_verify(
    Path(token): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    // Look the user up by the token; this lookup also checks it hasn't expired.
    let Ok(user) = users::Model::find_by_magic_token(&ctx.db, &token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return unauthorized("unauthorized!");
    };

    // The link is single-use: clear the token now so it can't be replayed.
    let user = user.into_active_model().clear_magic_link(&ctx.db).await?;

    // Issue a JWT, exactly like a normal login does.
    let jwt_secret = ctx.config.get_jwt_config()?;

    let token = user
        .generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    format::json(LoginResponse::new(&user, &token))
}

/// Re-send the verification email to a user who hasn't verified yet.
#[debug_handler]
async fn resend_verification_email(
    State(ctx): State<AppContext>,
    Json(params): Json<ResendVerificationParams>,
) -> Result<Response> {
    // Unknown email → silently return 200 (don't reveal who is registered).
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        tracing::info!(
            email = params.email,
            "User not found for resend verification"
        );
        return format::json(());
    };

    // Already verified → nothing to do, also return 200.
    if user.email_verified_at.is_some() {
        tracing::info!(
            pid = user.pid.to_string(),
            "User already verified, skipping resend"
        );
        return format::json(());
    }

    // Otherwise regenerate the verification token and re-send the welcome email.
    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;
    tracing::info!(pid = user.pid.to_string(), "Verification email re-sent");

    format::json(())
}

/// Connect each URL to its handler. The `.prefix("/api/auth")` means every path
/// below is relative to that — e.g. `register` is actually `POST /api/auth/register`.
/// `post`/`get` indicate which HTTP method each route accepts.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth")
        .add("/register", post(register))
        .add("/verify/{token}", get(verify))
        .add("/login", post(login))
        .add("/forgot", post(forgot))
        .add("/reset", post(reset))
        .add("/current", get(current))
        .add("/magic-link", post(magic_link))
        .add("/magic-link/{token}", get(magic_link_verify))
        .add("/resend-verification-mail", post(resend_verification_email))
}
