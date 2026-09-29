//! The login page's sign-in fields, reported so the web player can mirror them as
//! real inputs that the admin's password manager can fill.
//!
//! Scripts run in an isolated world: the page's own scripts cannot see them,
//! and they only read the page, apart from focusing a field to fill it.

use std::{fmt, time::Duration};

use chromiumoxide::{
    Page,
    cdp::{
        browser_protocol::page::CreateIsolatedWorldParams,
        js_protocol::runtime::{EvaluateParams, ExecutionContextId},
    },
};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::BrowserError;

/// How often the page is searched for fields.
const POLL: Duration = Duration::from_millis(400);

/// What a field asks for, as password managers understand it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Username,
    Password,
    OneTimeCode,
}

/// A sign-in field on the page, positioned in page CSS pixels (the space
/// frames and input use).
#[derive(Clone, PartialEq, Serialize)]
pub struct Field {
    /// Finds the field again; see [`Input::Fill`](crate::Input::Fill).
    pub key: String,
    pub kind: FieldKind,
    /// What the page calls it, e.g. "Email or phone".
    pub label: String,
    /// What it holds. Never reported for passwords.
    pub value: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl fmt::Debug for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("key", &self.key)
            .field("kind", &self.kind)
            .field("label", &self.label)
            .field("rect", &(self.x, self.y, self.width, self.height))
            .finish_non_exhaustive()
    }
}

/// A visible text input, as the discovery script reports it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawField {
    index: usize,
    r#type: String,
    autocomplete: String,
    name: String,
    id: String,
    label: String,
    value: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Script helpers shared by discovery and focusing.
const PRELUDE: &str = r#"
const pixiuVisible = (el) => {
  if (el.disabled || el.readOnly) return false;
  if (el.checkVisibility && !el.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) return false;
  const r = el.getBoundingClientRect();
  return r.width >= 8 && r.height >= 8 && r.bottom > 0 && r.right > 0 && r.top < innerHeight && r.left < innerWidth;
};
const pixiuTextual = (el) => ["text", "email", "password", "tel", "number", "search", ""].includes((el.getAttribute("type") || "").toLowerCase());
"#;

/// Lists the visible text inputs of the page.
const DISCOVER: &str = r#"
(() => [...document.querySelectorAll("input")].flatMap((el, index) => {
  if (!pixiuTextual(el) || !pixiuVisible(el)) return [];
  const type = (el.getAttribute("type") || "text").toLowerCase();
  const r = el.getBoundingClientRect();
  const label = (el.labels && el.labels[0] && el.labels[0].innerText) || el.getAttribute("aria-label") || el.placeholder || "";
  return [{
    index, type,
    autocomplete: el.getAttribute("autocomplete") || "",
    name: el.name || "", id: el.id || "",
    label: label.trim().slice(0, 80),
    value: type === "password" ? "" : el.value,
    x: r.x, y: r.y, width: r.width, height: r.height,
  }];
}))()
"#;

/// Focuses the field with the key passed in, reporting whether it exists.
const FOCUS: &str = r#"
((key) => {
  const split = key.indexOf(":");
  const by = key.slice(0, split), wanted = key.slice(split + 1);
  const inputs = [...document.querySelectorAll("input")];
  const el = by === "index"
    ? inputs[Number(wanted)]
    : inputs.find((i) => (by === "id" ? i.id : i.name) === wanted && pixiuTextual(i) && pixiuVisible(i));
  if (!el) return false;
  el.scrollIntoView({ block: "nearest", inline: "nearest" });
  el.focus({ preventScroll: true });
  return el === document.activeElement;
})"#;

/// Whether an input is a sign-in field worth mirroring, and which kind.
fn classify(raw: &RawField) -> Option<FieldKind> {
    let autocomplete = raw.autocomplete.to_ascii_lowercase();
    let tokens: Vec<&str> = autocomplete.split_whitespace().collect();
    let names = format!("{} {}", raw.name, raw.id).to_ascii_lowercase();
    let named = |hints: &[&str]| hints.iter().any(|hint| names.contains(hint));
    if raw.r#type == "password" {
        Some(FieldKind::Password)
    } else if tokens.contains(&"one-time-code") || named(&["otp", "totp", "pin", "code"]) {
        Some(FieldKind::OneTimeCode)
    } else if raw.r#type == "email"
        || tokens
            .iter()
            .any(|token| matches!(*token, "username" | "email" | "tel"))
        || named(&["user", "email", "login", "identifier", "account", "phone"])
    {
        Some(FieldKind::Username)
    } else {
        None
    }
}

/// A key that finds the same field again: its id or name when it has one,
/// else its position among the page's inputs.
fn key(raw: &RawField) -> String {
    if !raw.id.is_empty() {
        format!("id:{}", raw.id)
    } else if !raw.name.is_empty() {
        format!("name:{}", raw.name)
    } else {
        format!("index:{}", raw.index)
    }
}

fn fields(found: serde_json::Value) -> Result<Vec<Field>, BrowserError> {
    let raw: Vec<RawField> =
        serde_json::from_value(found).map_err(|error| BrowserError::Script(error.to_string()))?;
    Ok(raw
        .into_iter()
        .filter_map(|raw| {
            let kind = classify(&raw)?;
            Some(Field {
                key: key(&raw),
                kind,
                value: (kind != FieldKind::Password).then(|| raw.value.clone()),
                label: raw.label,
                x: raw.x,
                y: raw.y,
                width: raw.width,
                height: raw.height,
            })
        })
        .collect())
}

/// An isolated world in the page's main frame. Navigations destroy it; it
/// is made again when needed.
#[derive(Default)]
pub(crate) struct World {
    context: Option<ExecutionContextId>,
}

impl World {
    async fn evaluate(
        &mut self,
        page: &Page,
        expression: &str,
    ) -> Result<serde_json::Value, BrowserError> {
        let mut retried = false;
        loop {
            let context = match self.context {
                Some(context) => context,
                None => {
                    let context = create_world(page).await?;
                    self.context = Some(context);
                    context
                }
            };
            // A function scope per evaluation: the world outlives it, and
            // would reject the helpers being declared twice.
            let params = EvaluateParams::builder()
                .expression(format!(
                    "(() => {{ {PRELUDE}\nreturn {}; }})()",
                    expression.trim()
                ))
                .context_id(context)
                .return_by_value(true)
                .build()
                .map_err(BrowserError::Script)?;
            match page.execute(params).await {
                Ok(response) => {
                    if let Some(exception) = &response.result.exception_details {
                        return Err(BrowserError::Script(exception.text.clone()));
                    }
                    return Ok(response.result.result.value.clone().unwrap_or_default());
                }
                // Most likely a navigation took the world away.
                Err(_) if !retried => {
                    retried = true;
                    self.context = None;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    /// Focuses the field `key` names; `false` when the page has no such
    /// field (any more).
    pub(crate) async fn focus(&mut self, page: &Page, key: &str) -> Result<bool, BrowserError> {
        let key =
            serde_json::to_string(key).map_err(|error| BrowserError::Script(error.to_string()))?;
        let focused = self.evaluate(page, &format!("{FOCUS}({key})")).await?;
        Ok(focused.as_bool().unwrap_or(false))
    }
}

async fn create_world(page: &Page) -> Result<ExecutionContextId, BrowserError> {
    let frame = page
        .mainframe()
        .await?
        .ok_or_else(|| BrowserError::Script("the page has no main frame yet".to_owned()))?;
    let params = CreateIsolatedWorldParams::builder()
        .frame_id(frame)
        .world_name("pixiu")
        .build()
        .map_err(BrowserError::Script)?;
    Ok(page.execute(params).await?.result.execution_context_id)
}

/// Keeps `fields` up to date with the page's sign-in fields, until aborted.
pub(crate) async fn track(page: Page, found: watch::Sender<Vec<Field>>) {
    let mut world = World::default();
    loop {
        tokio::time::sleep(POLL).await;
        match world.evaluate(&page, DISCOVER).await.and_then(fields) {
            Ok(current) => {
                found.send_if_modified(|previous| {
                    let changed = *previous != current;
                    if changed {
                        *previous = current;
                    }
                    changed
                });
            }
            Err(error) => tracing::debug!(%error, "cannot read the login page's fields"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(r#type: &str, autocomplete: &str, name: &str, id: &str) -> RawField {
        RawField {
            index: 3,
            r#type: r#type.to_owned(),
            autocomplete: autocomplete.to_owned(),
            name: name.to_owned(),
            id: id.to_owned(),
            label: String::new(),
            value: "secret".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        }
    }

    #[test]
    fn google_sign_in_fields_are_recognized() {
        let identifier = raw("email", "username webauthn", "identifier", "identifierId");
        assert_eq!(classify(&identifier), Some(FieldKind::Username));
        assert_eq!(key(&identifier), "id:identifierId");

        let password = raw("password", "current-password", "Passwd", "");
        assert_eq!(classify(&password), Some(FieldKind::Password));
        assert_eq!(key(&password), "name:Passwd");

        assert_eq!(
            classify(&raw("tel", "off", "totpPin", "totpPin")),
            Some(FieldKind::OneTimeCode)
        );
        assert_eq!(
            classify(&raw("text", "one-time-code", "", "")),
            Some(FieldKind::OneTimeCode)
        );
        // A search box is not a sign-in field.
        let search = raw("text", "off", "q", "");
        assert_eq!(classify(&search), None);
        assert_eq!(key(&raw("text", "", "", "")), "index:3");
    }

    #[test]
    fn passwords_are_never_reported() {
        let found = serde_json::json!([
            {
                "index": 0, "type": "email", "autocomplete": "username", "name": "", "id": "u",
                "label": "Email", "value": "keeper@example.com", "x": 1, "y": 2, "width": 300,
                "height": 40
            },
            {
                "index": 1, "type": "password", "autocomplete": "current-password", "name": "pw",
                "id": "", "label": "Password", "value": "hunter2", "x": 1, "y": 60, "width": 300,
                "height": 40
            }
        ]);
        let fields = fields(found).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].value.as_deref(), Some("keeper@example.com"));
        assert_eq!(fields[1].kind, FieldKind::Password);
        assert_eq!(fields[1].value, None);
        assert!(!format!("{:?}", fields[0]).contains("keeper@example.com"));
        let json = serde_json::to_value(&fields[1]).unwrap();
        assert_eq!(json["kind"], "password");
        assert_eq!(json["value"], serde_json::Value::Null);
    }
}
