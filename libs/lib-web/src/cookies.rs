use crate::Result;
use lib_core::model::session::{SessionForAuth, SessionType};
use lib_utils::time::parse_utc;
use tower_cookies::cookie::{CookieBuilder, SameSite};
use tower_cookies::{Cookie, Cookies};

pub const SESSION_TOKEN: &str = "session-token";

pub fn set_session_cookie(cookies: &Cookies, session_auth: SessionForAuth) -> Result<()> {
    let SessionForAuth {
        token,
        privileged,
        expiration,
    } = session_auth;

    let mut cookie = CookieBuilder::new(SESSION_TOKEN, token)
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .build();

    // Add expiration only in case of a non privileged session
    if !privileged {
        cookie.set_expires(parse_utc(&expiration)?);
    }

    cookies.add(cookie);

    Ok(())
}

pub fn remove_session_cookie(cookies: &Cookies) -> Result<()> {
    let mut cookie = Cookie::from(SESSION_TOKEN);
    cookie.set_path("/");

    cookies.remove(cookie);

    Ok(())
}
