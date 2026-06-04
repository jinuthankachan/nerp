//! Auth mailer — builds and sends the account-related emails (welcome/verify,
//! forgot-password, and magic-link).
//!
//! Each method picks an email template folder, fills in some per-user values
//! ("locals"), and hands it to loco's mailer to render and deliver.

// auth mailer
#![allow(non_upper_case_globals)] // allow lowercase `static` names like `welcome`

use loco_rs::prelude::*;
use serde_json::json; // the `json!{...}` macro for building JSON values

use crate::models::users;

// `include_dir!` bundles these template folders *into the compiled binary* at
// build time, so the running program doesn't need the files on disk. Each
// folder holds the subject/html/text templates for one email.
static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
static forgot: Dir<'_> = include_dir!("src/mailers/auth/forgot");
static magic_link: Dir<'_> = include_dir!("src/mailers/auth/magic_link");

// An empty struct used only to group the sending functions below.
#[allow(clippy::module_name_repetitions)]
pub struct AuthMailer {}
impl Mailer for AuthMailer {} // opt in to loco's mailer helpers
impl AuthMailer {
    /// Sending welcome email the the given user
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_welcome(ctx: &AppContext, user: &users::Model) -> Result<()> {
        // Render the `welcome` template and send it to the user's address. The
        // `locals` are the variables the template can reference (name, the
        // verification token to build a confirm link, and the site URL).
        Self::mail_template(
            ctx,
            &welcome,
            mailer::Args {
                to: user.email.to_string(),
                locals: json!({
                  "name": user.name,
                  "verifyToken": user.email_verification_token,
                  "domain": ctx.config.server.full_url()
                }),
                ..Default::default() // leave all other mail options at defaults
            },
        )
        .await?;

        Ok(())
    }

    /// Sending forgot password email
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn forgot_password(ctx: &AppContext, user: &users::Model) -> Result<()> {
        Self::mail_template(
            ctx,
            &forgot,
            mailer::Args {
                to: user.email.to_string(),
                locals: json!({
                  "name": user.name,
                  "resetToken": user.reset_token, // used to build the reset link
                  "domain": ctx.config.server.full_url()
                }),
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }

    /// Sends a magic link authentication email to the user.
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_magic_link(ctx: &AppContext, user: &users::Model) -> Result<()> {
        Self::mail_template(
            ctx,
            &magic_link,
            mailer::Args {
                to: user.email.to_string(),
                locals: json!({
                  "name": user.name,
                  // The token must be present to build the login link. If it's
                  // somehow missing, `ok_or_else` turns that into an error
                  // (via `?`) instead of sending a broken email.
                  "token": user.magic_link_token.clone().ok_or_else(|| Error::string(
                            "the user model not contains magic link token",
                    ))?,
                  "host": ctx.config.server.full_url()
                }),
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }
}
