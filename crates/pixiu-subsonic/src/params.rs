//! Request parameters.
//!
//! Subsonic clients send parameters in the query string, or, with the
//! OpenSubsonic `formPost` extension, as a form-encoded POST body. Keys may
//! repeat (`id=1&id=2`), so parameters are kept as an ordered list instead of
//! being deserialized into a struct.

use crate::response::{ApiError, Format};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Params(Vec<(String, String)>);

impl Params {
    /// Merges the query string and a form-encoded body, query first.
    #[must_use]
    pub fn parse(query: Option<&str>, form_body: Option<&[u8]>) -> Self {
        let mut pairs = Vec::new();
        if let Some(query) = query {
            pairs.extend(form_urlencoded::parse(query.as_bytes()).into_owned());
        }
        if let Some(body) = form_body {
            pairs.extend(form_urlencoded::parse(body).into_owned());
        }
        Self(pairs)
    }

    /// The first value of `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Every value of `name`, in request order.
    pub fn get_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.0
            .iter()
            .filter(move |(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The first value of `name`, or the Subsonic "missing parameter" error.
    ///
    /// # Errors
    ///
    /// Fails when the parameter is absent.
    pub fn require(&self, name: &str) -> Result<&str, ApiError> {
        self.get(name)
            .ok_or_else(|| ApiError::missing_parameter(name))
    }

    /// The response format requested with `f`. Unknown formats fall back to
    /// XML, the Subsonic default.
    #[must_use]
    pub fn format(&self) -> Format {
        match self.get("f") {
            Some("json") => Format::Json,
            Some("jsonp") => match self.get("callback") {
                Some(callback) if is_js_identifier(callback) => Format::Jsonp(callback.to_owned()),
                _ => Format::Json,
            },
            _ => Format::Xml,
        }
    }
}

/// Guards JSONP against script injection through the callback name.
fn is_js_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_query_and_body_keeping_repeats() {
        let params = Params::parse(Some("u=admin&id=1&id=2"), Some(b"id=3&f=json"));
        assert_eq!(params.get("u"), Some("admin"));
        assert_eq!(params.get_all("id").collect::<Vec<_>>(), ["1", "2", "3"]);
        assert_eq!(params.format(), Format::Json);
    }

    #[test]
    fn decodes_percent_encoding() {
        let params = Params::parse(Some("query=AC%2FDC&title=a+b"), None);
        assert_eq!(params.get("query"), Some("AC/DC"));
        assert_eq!(params.get("title"), Some("a b"));
    }

    #[test]
    fn missing_parameters_are_reported() {
        let error = Params::default().require("id").unwrap_err();
        assert_eq!(error.code, crate::response::ErrorCode::MissingParameter);
    }

    #[test]
    fn jsonp_callbacks_are_validated() {
        let ok = Params::parse(Some("f=jsonp&callback=app.cb_1"), None);
        assert_eq!(ok.format(), Format::Jsonp("app.cb_1".to_owned()));

        let evil = Params::parse(Some("f=jsonp&callback=alert(1)//"), None);
        assert_eq!(evil.format(), Format::Json);
    }
}
