//! Carries the WebUI session token in a cookie that also works over plain
//! HTTP.
//!
//! Topcoat's `CookieTokenStore` always marks the cookie `Secure` with a
//! `__Host-` prefix. Browsers refuse such cookies over plain HTTP, which is how
//! most self-hosted instances are first reached (`http://nas.local:4533`), so
//! the hardening here follows [`CookieSecurity`].

use std::time::Duration;

use pixiu_core::CookieSecurity;
use topcoat::{
    context::Cx,
    cookie::{Cookie, Cookies, SameSite, cookies},
    router::request::{headers, uri},
    session::{Token, TokenStore, TokenStoreFuture},
};

const COOKIE_NAME: &str = "pixiu_session";

pub struct PixiuCookieStore {
    security: CookieSecurity,
}

impl PixiuCookieStore {
    #[must_use]
    pub fn new(security: CookieSecurity) -> Self {
        Self { security }
    }

    fn hardened(&self, cx: &Cx) -> bool {
        match self.security {
            CookieSecurity::Always => true,
            CookieSecurity::Never => false,
            CookieSecurity::Auto => is_https(cx),
        }
    }
}

/// Whether the client reached us over HTTPS, directly or through a proxy.
///
/// Trusting `X-Forwarded-Proto` blindly is fine here: a spoofed header can
/// only make the client's own cookie stricter.
fn is_https(cx: &Cx) -> bool {
    uri(cx).scheme_str() == Some("https")
        || headers(cx)
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|proto| proto.eq_ignore_ascii_case("https"))
}

fn base(cx: &Cx) -> impl Cookies {
    cookies(cx)
        .override_same_site(SameSite::Lax)
        .override_http_only(true)
        .override_path("/")
}

impl TokenStore for PixiuCookieStore {
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
        Box::pin(async move {
            let cookie = if self.hardened(cx) {
                base(cx)
                    .override_secure(true)
                    .override_prefix_host()
                    .get(COOKIE_NAME)
            } else {
                base(cx).get(COOKIE_NAME)
            };
            Ok(cookie.and_then(|cookie| Token::decode(cookie.value_trimmed()).ok()))
        })
    }

    fn write<'a>(
        &'a self,
        cx: &'a Cx,
        token: Token,
        max_age: Duration,
    ) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let max_age = topcoat::cookie::time::Duration::try_from(max_age)?;
            let cookie = Cookie::new(COOKIE_NAME, token.encode());
            if self.hardened(cx) {
                base(cx)
                    .override_secure(true)
                    .override_prefix_host()
                    .override_max_age(max_age)
                    .add(cookie);
            } else {
                base(cx).override_max_age(max_age).add(cookie);
            }
            Ok(())
        })
    }

    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let cookie = Cookie::new(COOKIE_NAME, "");
            if self.hardened(cx) {
                base(cx)
                    .override_secure(true)
                    .override_prefix_host()
                    .remove(cookie);
            } else {
                base(cx).remove(cookie);
            }
            Ok(())
        })
    }
}
