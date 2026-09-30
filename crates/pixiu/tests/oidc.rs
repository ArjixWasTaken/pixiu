//! Single sign-on, against a mock OpenID Connect provider served in the
//! test: linking accounts, signing in with them, and every way a sign-in
//! must fail (wrong browser, replayed state, bad tokens, unlinked or
//! disabled accounts).

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};

use axum::{
    Form, Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderMap, Method, Request, StatusCode, header},
    routing::{get, post},
};
use openidconnect::{
    Audience, EmptyAdditionalClaims, EndUserEmail, IssuerUrl, JsonWebKeyId, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, PrivateSigningKey, StandardClaims, SubjectIdentifier,
    core::{
        CoreIdToken, CoreIdTokenClaims, CoreJsonWebKeySet, CoreJwsSigningAlgorithm,
        CoreRsaPrivateSigningKey,
    },
};
use pixiu_core::{Config, SecretBox};
use serde::Deserialize;
use serde_json::{Value, json};
use tower::ServiceExt;

const CLIENT_ID: &str = "pixiu";
const CLIENT_SECRET: &str = "the client secret";

fn key(name: &str) -> CoreRsaPrivateSigningKey {
    let pem = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/oidc")
            .join(format!("{name}.pem")),
    )
    .unwrap();
    CoreRsaPrivateSigningKey::from_pem(&pem, Some(JsonWebKeyId::new(name.to_owned()))).unwrap()
}

/// Makes a grant's token go wrong in one way.
type Spoil = fn(&mut Grant);

/// Who the mock provider says signed in, and how it spoils the token.
#[derive(Clone)]
struct Grant {
    subject: String,
    email: String,
    nonce: String,
    challenge: String,
    audience: String,
    issuer: Option<String>,
    expired: bool,
}

struct Provider {
    issuer: String,
    /// The key it signs with, and the keys it publishes.
    signing: Mutex<String>,
    published: Mutex<Vec<String>>,
    grants: Mutex<HashMap<String, Grant>>,
}

async fn discovery(State(provider): State<Arc<Provider>>) -> Json<Value> {
    let issuer = &provider.issuer;
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "jwks_uri": format!("{issuer}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "token_endpoint_auth_methods_supported": ["client_secret_basic"],
        "code_challenge_methods_supported": ["S256"],
    }))
}

async fn jwks(State(provider): State<Arc<Provider>>) -> Json<CoreJsonWebKeySet> {
    let keys = provider
        .published
        .lock()
        .unwrap()
        .iter()
        .map(|name| key(name).as_verification_key())
        .collect();
    Json(CoreJsonWebKeySet::new(keys))
}

#[derive(Deserialize)]
struct TokenRequest {
    grant_type: String,
    code: String,
    code_verifier: String,
}

async fn token(
    State(provider): State<Arc<Provider>>,
    headers: HeaderMap,
    Form(request): Form<TokenRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let refuse = |error: &str| (StatusCode::BAD_REQUEST, Json(json!({ "error": error })));
    // Client authentication, as the discovery document says: basic.
    let basic = format!(
        "Basic {}",
        base64_basic(&format!("{CLIENT_ID}:{}", form_encode(CLIENT_SECRET)))
    );
    if headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        != Some(&basic)
    {
        return Err(refuse("invalid_client"));
    }
    let grant = provider
        .grants
        .lock()
        .unwrap()
        .remove(&request.code)
        .filter(|_| request.grant_type == "authorization_code")
        .ok_or_else(|| refuse("invalid_grant"))?;
    let challenge =
        PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(request.code_verifier));
    if challenge.as_str() != grant.challenge {
        return Err(refuse("invalid_grant"));
    }
    let now = chrono::Utc::now();
    let (issued, expires) = if grant.expired {
        (
            now - chrono::Duration::hours(2),
            now - chrono::Duration::hours(1),
        )
    } else {
        (now, now + chrono::Duration::minutes(5))
    };
    let claims = CoreIdTokenClaims::new(
        IssuerUrl::new(grant.issuer.unwrap_or_else(|| provider.issuer.clone())).unwrap(),
        vec![Audience::new(grant.audience)],
        expires,
        issued,
        StandardClaims::new(SubjectIdentifier::new(grant.subject))
            .set_email(Some(EndUserEmail::new(grant.email))),
        EmptyAdditionalClaims {},
    )
    .set_nonce(Some(Nonce::new(grant.nonce)));
    let signing = provider.signing.lock().unwrap().clone();
    let id_token = CoreIdToken::new(
        claims,
        &key(&signing),
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        None,
        None,
    )
    .unwrap();
    Ok(Json(json!({
        "access_token": "an access token",
        "token_type": "Bearer",
        "expires_in": 300,
        "id_token": id_token.to_string(),
    })))
}

/// `application/x-www-form-urlencoded`, as OAuth 2 wants client
/// credentials encoded before they are joined for basic authentication.
fn form_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'*' => {
                (byte as char).to_string()
            }
            b' ' => "+".to_owned(),
            byte => format!("%{byte:02X}"),
        })
        .collect()
}

fn base64_basic(value: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = value.as_bytes();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Starts the mock provider; it answers at its issuer URL.
async fn provider() -> Arc<Provider> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address: SocketAddr = listener.local_addr().unwrap();
    let provider = Arc::new(Provider {
        issuer: format!("http://{address}"),
        signing: Mutex::new("key1".to_owned()),
        published: Mutex::new(vec!["key1".to_owned()]),
        grants: Mutex::default(),
    });
    let app = Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks", get(jwks))
        .route("/token", post(token))
        .with_state(Arc::clone(&provider));
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    provider
}

struct Server {
    _dir: tempfile::TempDir,
    router: Router,
    provider: Arc<Provider>,
}

/// What the callback answered: where it sends the browser.
struct Back {
    location: String,
}

impl Back {
    /// The `/sso/<code>` code, if it is one.
    fn code(&self) -> Option<&str> {
        self.location.strip_prefix("/sso/")
    }
}

/// A sign-in on its way: what píxiū sent the browser to the provider with.
struct Leaving {
    state: String,
    nonce: String,
    challenge: String,
    cookie: String,
}

fn query_of(url: &str) -> HashMap<String, String> {
    url.split_once('?')
        .map(|(_, query)| query)
        .unwrap_or_default()
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_owned(), decode(value)))
        .collect()
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                out.push(u8::from_str_radix(&value[i + 1..i + 3], 16).unwrap());
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap()
}

impl Server {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.paths.data_dir = dir.path().join("data");
        config.paths.treasure_dir = dir.path().join("treasure");
        let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let services = pixiu::Services::new(db, &config, SecretBox::ephemeral())
            .await
            .unwrap();
        let web = dir.path().join("web");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(web.join("index.html"), "<!doctype html>").unwrap();
        Self {
            router: pixiu::app(&services, &config, &web),
            _dir: dir,
            provider: provider().await,
        }
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, HeaderMap, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let (status, headers) = (response.status(), response.headers().clone());
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            headers,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn call(
        &self,
        token: Option<&str>,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        }
        .unwrap();
        let (status, _, body) = self.send(request).await;
        (status, body)
    }

    /// Sets píxiū up with Alice as admin, and the provider; her token.
    async fn setup(&self) -> String {
        let (status, body) = self
            .call(
                None,
                Method::POST,
                "/api/auth/setup",
                Some(json!({ "username": "alice", "password": "alice's secret" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let alice = body["token"].as_str().unwrap().to_owned();
        let (status, _) = self
            .call(
                Some(&alice),
                Method::PUT,
                "/api/admin/settings/server",
                Some(json!({ "public_url": "https://music.example.com" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = self
            .call(
                Some(&alice),
                Method::PUT,
                "/api/admin/settings/oidc",
                Some(json!({
                    "name": "Mock",
                    "issuer": self.provider.issuer,
                    "client_id": CLIENT_ID,
                    // Pasted, with what came along.
                    "client_secret": format!(" {CLIENT_SECRET}\n"),
                })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        alice
    }

    /// Makes an account; its token.
    async fn user(&self, admin: &str, name: &str) -> String {
        let (status, body) = self
            .call(
                Some(admin),
                Method::POST,
                "/api/admin/users",
                Some(json!({ "username": name, "password": "a temporary one" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = self
            .call(
                None,
                Method::POST,
                "/api/auth/login",
                Some(json!({ "username": name, "password": "a temporary one" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }

    fn leaving(location: &str, headers: &HeaderMap) -> Leaving {
        let query = query_of(location);
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(query["client_id"], CLIENT_ID);
        assert_eq!(
            query["redirect_uri"],
            "https://music.example.com/api/auth/oidc/callback"
        );
        assert!(query["scope"].contains("openid"), "{}", query["scope"]);
        let cookie = headers[header::SET_COOKIE].to_str().unwrap();
        assert!(
            cookie.contains("HttpOnly") && cookie.contains("Secure"),
            "{cookie}"
        );
        Leaving {
            state: query["state"].clone(),
            nonce: query["nonce"].clone(),
            challenge: query["code_challenge"].clone(),
            cookie: cookie.split(';').next().unwrap().to_owned(),
        }
    }

    /// Starts signing in.
    async fn start(&self) -> Leaving {
        let (status, headers, _) = self
            .send(
                Request::get("/api/auth/oidc/start")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let location = headers[header::LOCATION].to_str().unwrap();
        assert!(
            location.starts_with(&format!("{}/authorize?", self.provider.issuer)),
            "{location}"
        );
        Self::leaving(location, &headers)
    }

    /// Starts linking an account to `token`'s.
    async fn start_link(&self, token: &str) -> Leaving {
        let (status, headers, body) = self
            .send(
                Request::post("/api/me/identities/oidc")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        Self::leaving(body["url"].as_str().unwrap(), &headers)
    }

    /// The provider lets `subject` in, as `grant` spoils it.
    fn authorize(
        &self,
        leaving: &Leaving,
        subject: &str,
        spoil: impl FnOnce(&mut Grant),
    ) -> String {
        let mut grant = Grant {
            subject: subject.to_owned(),
            email: format!("{subject}@sso.example.com"),
            nonce: leaving.nonce.clone(),
            challenge: leaving.challenge.clone(),
            audience: CLIENT_ID.to_owned(),
            issuer: None,
            expired: false,
        };
        spoil(&mut grant);
        let code = format!("code-{}", leaving.state);
        self.provider
            .grants
            .lock()
            .unwrap()
            .insert(code.clone(), grant);
        code
    }

    /// Comes back from the provider with `query`, and the cookie.
    async fn back(&self, query: &str, cookie: Option<&str>) -> Back {
        let mut request = Request::get(format!("/api/auth/oidc/callback?{query}"));
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, format!("other=1; {cookie}"));
        }
        let (status, headers, _) = self.send(request.body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let cleared = headers[header::SET_COOKIE].to_str().unwrap();
        assert!(cleared.contains("Max-Age=0"), "{cleared}");
        Back {
            location: headers[header::LOCATION].to_str().unwrap().to_owned(),
        }
    }

    /// Signs in as `subject` at the provider, all the way.
    async fn sign_in(&self, subject: &str) -> Back {
        let leaving = self.start().await;
        let code = self.authorize(&leaving, subject, |_| {});
        self.back(
            &format!("state={}&code={code}", leaving.state),
            Some(&leaving.cookie),
        )
        .await
    }

    /// Links `subject` at the provider to `token`'s account.
    async fn link(&self, token: &str, subject: &str) -> Back {
        let leaving = self.start_link(token).await;
        let code = self.authorize(&leaving, subject, |_| {});
        self.back(
            &format!("state={}&code={code}", leaving.state),
            Some(&leaving.cookie),
        )
        .await
    }

    async fn exchange(&self, code: &str) -> (StatusCode, Value) {
        self.call(
            None,
            Method::POST,
            "/api/auth/oidc/exchange",
            Some(json!({ "code": code })),
        )
        .await
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn linked_accounts_sign_in() {
    let server = Server::new().await;
    let alice = server.setup().await;
    let (_, status) = server
        .call(None, Method::GET, "/api/auth/status", None)
        .await;
    assert_eq!(status["sso"], json!({ "name": "Mock" }));

    // Linking.
    let back = server.link(&alice, "alice-at-sso").await;
    assert_eq!(back.location, "/settings?tab=account&linked=1");
    let (_, links) = server
        .call(Some(&alice), Method::GET, "/api/me/identities", None)
        .await;
    assert_eq!(links[0]["provider"], "Mock");
    assert_eq!(links[0]["email"], "alice-at-sso@sso.example.com");

    // Signing in: a code, traded once for a token.
    let back = server.sign_in("alice-at-sso").await;
    let code = back.code().expect("a code").to_owned();
    let (status, tokens) = server.exchange(&code).await;
    assert_eq!(status, StatusCode::OK, "{tokens}");
    let (status, me) = server
        .call(tokens["token"].as_str(), Method::GET, "/api/me", None)
        .await;
    assert_eq!((status, &me["username"]), (StatusCode::OK, &json!("alice")));
    let (status, body) = server.exchange(&code).await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::GONE, &json!("expired"))
    );
    let (_, links) = server
        .call(Some(&alice), Method::GET, "/api/me/identities", None)
        .await;
    assert!(links[0]["last_login_at"].is_string(), "{links}");

    // Unlinked, the account there no longer signs in here.
    let id = links[0]["id"].as_u64().unwrap();
    let (status, _) = server
        .call(
            Some(&alice),
            Method::DELETE,
            &format!("/api/me/identities/{id}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        server.sign_in("alice-at-sso").await.location,
        "/?sso_error=unlinked"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nobody_signs_in_without_a_link() {
    let server = Server::new().await;
    let alice = server.setup().await;
    assert_eq!(
        server.sign_in("stranger").await.location,
        "/?sso_error=unlinked"
    );
    let (_, users) = server
        .call(Some(&alice), Method::GET, "/api/admin/users", None)
        .await;
    assert_eq!(users.as_array().unwrap().len(), 1, "no account was made");

    // A turned-off account does not sign in either.
    let bob = server.user(&alice, "bob").await;
    server.link(&bob, "bob-at-sso").await;
    let (_, users) = server
        .call(Some(&alice), Method::GET, "/api/admin/users", None)
        .await;
    let bob_id = users[1]["id"].as_u64().unwrap();
    server
        .call(
            Some(&alice),
            Method::PATCH,
            &format!("/api/admin/users/{bob_id}"),
            Some(json!({ "status": "disabled" })),
        )
        .await;
    assert_eq!(
        server.sign_in("bob-at-sso").await.location,
        "/?sso_error=disabled"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sign_ins_belong_to_the_browser_that_started_them() {
    let server = Server::new().await;
    let alice = server.setup().await;
    server.link(&alice, "alice-at-sso").await;

    // No cookie, or another browser's: refused, and used up.
    for cookie in [None, Some("pixiu_oidc=someone-elses")] {
        let leaving = server.start().await;
        let code = server.authorize(&leaving, "alice-at-sso", |_| {});
        let query = format!("state={}&code={code}", leaving.state);
        assert_eq!(
            server.back(&query, cookie).await.location,
            "/?sso_error=expired"
        );
        assert_eq!(
            server.back(&query, Some(&leaving.cookie)).await.location,
            "/?sso_error=expired",
            "replayed"
        );
    }

    // A state píxiū never gave out.
    let leaving = server.start().await;
    assert_eq!(
        server
            .back("state=made-up&code=x", Some(&leaving.cookie))
            .await
            .location,
        "/?sso_error=expired"
    );

    // The provider said no.
    assert_eq!(
        server
            .back(
                &format!("state={}&error=access_denied", leaving.state),
                Some(&leaving.cookie)
            )
            .await
            .location,
        "/?sso_error=denied"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokens_are_checked() {
    let server = Server::new().await;
    let alice = server.setup().await;
    server.link(&alice, "alice-at-sso").await;
    let spoiled: [(&str, Spoil); 4] = [
        ("nonce", |grant| grant.nonce = "another".to_owned()),
        ("audience", |grant| {
            grant.audience = "another-client".to_owned()
        }),
        ("issuer", |grant| {
            grant.issuer = Some("https://evil.example.com".to_owned());
        }),
        ("expiry", |grant| grant.expired = true),
    ];
    for (what, spoil) in spoiled {
        let leaving = server.start().await;
        let code = server.authorize(&leaving, "alice-at-sso", spoil);
        let back = server
            .back(
                &format!("state={}&code={code}", leaving.state),
                Some(&leaving.cookie),
            )
            .await;
        assert_eq!(back.location, "/?sso_error=failed", "{what}");
    }

    // A code for another verifier: the provider refuses it.
    let leaving = server.start().await;
    let code = server.authorize(&leaving, "alice-at-sso", |grant| {
        grant.challenge = "not-the-challenge".to_owned();
    });
    assert_eq!(
        server
            .back(
                &format!("state={}&code={code}", leaving.state),
                Some(&leaving.cookie)
            )
            .await
            .location,
        "/?sso_error=failed"
    );

    // A token signed with a key píxiū has not seen: it looks again, once.
    *server.provider.signing.lock().unwrap() = "key2".to_owned();
    server
        .provider
        .published
        .lock()
        .unwrap()
        .push("key2".to_owned());
    assert!(server.sign_in("alice-at-sso").await.code().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accounts_there_link_to_one_account_here() {
    let server = Server::new().await;
    let alice = server.setup().await;
    let bob = server.user(&alice, "bob").await;
    server.link(&alice, "alice-at-sso").await;

    assert_eq!(
        server.link(&bob, "alice-at-sso").await.location,
        "/settings?tab=account&link_error=taken"
    );
    let (_, links) = server
        .call(Some(&bob), Method::GET, "/api/me/identities", None)
        .await;
    assert_eq!(links, json!([]));

    // Linking again replaces the account's link there.
    server.link(&alice, "alice-elsewhere").await;
    let (_, links) = server
        .call(Some(&alice), Method::GET, "/api/me/identities", None)
        .await;
    assert_eq!(links.as_array().unwrap().len(), 1);
    assert_eq!(links[0]["email"], "alice-elsewhere@sso.example.com");
    assert_eq!(
        server.link(&bob, "alice-at-sso").await.location,
        "/settings?tab=account&linked=1"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admins_set_the_provider_up() {
    let server = Server::new().await;
    let alice = server.setup().await;
    let (_, settings) = server
        .call(Some(&alice), Method::GET, "/api/admin/settings", None)
        .await;
    assert_eq!(settings["oidc"]["secret_set"], true);
    assert_eq!(
        settings["redirect_uri"],
        "https://music.example.com/api/auth/oidc/callback"
    );
    assert!(!settings.to_string().contains(CLIENT_SECRET));

    // Testing, with the stored secret.
    let form = json!({
        "name": "Mock",
        "issuer": server.provider.issuer,
        "client_id": CLIENT_ID,
    });
    let (status, report) = server
        .call(
            Some(&alice),
            Method::POST,
            "/api/admin/settings/oidc/test",
            Some(form),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report, json!({ "keys": 1, "pkce_s256": true }));
    let (status, body) = server
        .call(
            Some(&alice),
            Method::POST,
            "/api/admin/settings/oidc/test",
            Some(json!({
                "name": "Nowhere",
                "issuer": "http://127.0.0.1:9",
                "client_id": CLIENT_ID,
            })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "provider");

    // Removed, the button goes, and sign-ins go home.
    let (status, _) = server
        .call(
            Some(&alice),
            Method::DELETE,
            "/api/admin/settings/oidc",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, status) = server
        .call(None, Method::GET, "/api/auth/status", None)
        .await;
    assert_eq!(status["sso"], Value::Null);
    let (_, headers, _) = server
        .send(
            Request::get("/api/auth/oidc/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(headers[header::LOCATION], "/?sso_error=unavailable");
}
