//! Subsonic responses, rendered as XML or JSON from one tree.
//!
//! Subsonic's two formats disagree on structure: XML uses attributes and
//! repeated child elements, JSON uses fields and arrays (even for a single
//! item). Building an [`Element`] tree and rendering it twice keeps the two in
//! sync without serde gymnastics.

use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use serde_json::{Map, Value as Json};

/// The Subsonic API version píxiū implements.
pub const API_VERSION: &str = "1.16.1";

const XML_NAMESPACE: &str = "http://subsonic.org/restapi";
const SERVER_TYPE: &str = "pixiu";

/// A scalar attribute value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl Value {
    fn to_xml(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
        }
    }

    fn to_json(&self) -> Json {
        match self {
            Value::Str(s) => Json::String(s.clone()),
            Value::Int(i) => Json::from(*i),
            Value::Float(f) => serde_json::Number::from_f64(*f).map_or(Json::Null, Json::Number),
            Value::Bool(b) => Json::Bool(*b),
        }
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Str(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Str(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}

macro_rules! int_value {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Value::Int(i64::from(value))
            }
        }
    )*};
}
int_value!(i8, i16, i32, i64, u8, u16, u32);

impl From<u64> for Value {
    /// Saturates: sizes and counts never approach `i64::MAX` in practice.
    fn from(value: u64) -> Self {
        Value::Int(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Child {
    /// A single nested element: `<genre/>` / `"genre": {}`.
    One(Element),
    /// A list of elements: repeated `<song/>` / `"song": [...]`.
    List(&'static str, Vec<Element>),
    /// A list of scalars: repeated `<versions>1</versions>` / `"versions": [1]`.
    Values(&'static str, Vec<Value>),
    /// A scalar child: `<notes>text</notes>` / `"notes": "text"`.
    Field(&'static str, Value),
}

/// A node of a response body.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    name: &'static str,
    attrs: Vec<(&'static str, Value)>,
    children: Vec<Child>,
    /// XML text content; the `value` field in JSON.
    text: Option<String>,
}

impl Element {
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            attrs: Vec::new(),
            children: Vec::new(),
            text: None,
        }
    }

    #[must_use]
    pub fn attr(mut self, key: &'static str, value: impl Into<Value>) -> Self {
        self.attrs.push((key, value.into()));
        self
    }

    /// Adds the attribute only when `value` is present.
    #[must_use]
    pub fn attr_opt<V: Into<Value>>(self, key: &'static str, value: Option<V>) -> Self {
        match value {
            Some(value) => self.attr(key, value),
            None => self,
        }
    }

    #[must_use]
    pub fn child(mut self, element: Element) -> Self {
        self.children.push(Child::One(element));
        self
    }

    /// Adds a list of elements named `name`. In JSON the list is always an
    /// array, even when it holds one element.
    #[must_use]
    pub fn list(mut self, name: &'static str, items: impl IntoIterator<Item = Element>) -> Self {
        self.children
            .push(Child::List(name, items.into_iter().collect()));
        self
    }

    #[must_use]
    pub fn values<V: Into<Value>>(
        mut self,
        name: &'static str,
        values: impl IntoIterator<Item = V>,
    ) -> Self {
        self.children.push(Child::Values(
            name,
            values.into_iter().map(Into::into).collect(),
        ));
        self
    }

    /// Adds a scalar rendered as a child element in XML and a plain field in
    /// JSON, like `musicBrainzId` in `artistInfo`.
    #[must_use]
    pub fn field(mut self, name: &'static str, value: impl Into<Value>) -> Self {
        self.children.push(Child::Field(name, value.into()));
        self
    }

    /// Adds the field only when `value` is present.
    #[must_use]
    pub fn field_opt<V: Into<Value>>(self, name: &'static str, value: Option<V>) -> Self {
        match value {
            Some(value) => self.field(name, value),
            None => self,
        }
    }

    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    fn write_xml(&self, writer: &mut Writer<Vec<u8>>) {
        let mut start = BytesStart::new(self.name);
        for (key, value) in &self.attrs {
            start.push_attribute((*key, value.to_xml().as_str()));
        }
        write_xml_contents(
            writer,
            start,
            self.name,
            &self.children,
            self.text.as_deref(),
        );
    }

    fn to_json(&self) -> Json {
        let mut map = Map::new();
        for (key, value) in &self.attrs {
            map.insert((*key).to_owned(), value.to_json());
        }
        children_to_json(&mut map, &self.children);
        if let Some(text) = &self.text {
            map.insert("value".to_owned(), Json::String(text.clone()));
        }
        Json::Object(map)
    }
}

fn write_xml_contents(
    writer: &mut Writer<Vec<u8>>,
    start: BytesStart<'_>,
    name: &str,
    children: &[Child],
    text: Option<&str>,
) {
    const INFALLIBLE: &str = "writing XML to a Vec cannot fail";

    if children.is_empty() && text.is_none() {
        writer.write_event(Event::Empty(start)).expect(INFALLIBLE);
        return;
    }

    writer.write_event(Event::Start(start)).expect(INFALLIBLE);
    for child in children {
        match child {
            Child::One(element) => element.write_xml(writer),
            Child::List(_, elements) => {
                for element in elements {
                    element.write_xml(writer);
                }
            }
            Child::Values(name, values) => {
                for value in values {
                    writer
                        .create_element(*name)
                        .write_text_content(BytesText::new(&value.to_xml()))
                        .expect(INFALLIBLE);
                }
            }
            Child::Field(name, value) => {
                writer
                    .create_element(*name)
                    .write_text_content(BytesText::new(&value.to_xml()))
                    .expect(INFALLIBLE);
            }
        }
    }
    if let Some(text) = text {
        writer
            .write_event(Event::Text(BytesText::new(text)))
            .expect(INFALLIBLE);
    }
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .expect(INFALLIBLE);
}

fn children_to_json(map: &mut Map<String, Json>, children: &[Child]) {
    for child in children {
        match child {
            Child::One(element) => {
                map.insert(element.name.to_owned(), element.to_json());
            }
            Child::List(name, elements) => {
                map.insert(
                    (*name).to_owned(),
                    Json::Array(elements.iter().map(Element::to_json).collect()),
                );
            }
            Child::Values(name, values) => {
                map.insert(
                    (*name).to_owned(),
                    Json::Array(values.iter().map(Value::to_json).collect()),
                );
            }
            Child::Field(name, value) => {
                map.insert((*name).to_owned(), value.to_json());
            }
        }
    }
}

/// Error codes defined by the (Open)Subsonic API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorCode {
    Generic = 0,
    MissingParameter = 10,
    ClientTooOld = 20,
    ServerTooOld = 30,
    WrongCredentials = 40,
    TokenAuthNotSupported = 41,
    AuthNotSupported = 42,
    ConflictingAuth = 43,
    InvalidApiKey = 44,
    NotAuthorized = 50,
    NotFound = 70,
}

/// A failed request, reported inside a `status="failed"` envelope.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}

impl ApiError {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn missing_parameter(name: &str) -> Self {
        Self::new(
            ErrorCode::MissingParameter,
            format!("required parameter `{name}` is missing"),
        )
    }
}

/// The wire format a client asked for with the `f` parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Xml,
    Json,
    /// JSON wrapped in a call to the named callback.
    Jsonp(String),
}

/// What a successful response carries inside its envelope.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Payload(Vec<Child>);

impl Payload {
    /// A payload holding one element, e.g. `<album>`.
    #[must_use]
    pub fn element(element: Element) -> Self {
        Self(vec![Child::One(element)])
    }

    /// A payload holding a list directly under the envelope, e.g.
    /// `openSubsonicExtensions`.
    #[must_use]
    pub fn list(name: &'static str, items: impl IntoIterator<Item = Element>) -> Self {
        Self(vec![Child::List(name, items.into_iter().collect())])
    }
}

impl From<Element> for Payload {
    fn from(element: Element) -> Self {
        Self::element(element)
    }
}

/// A complete response: the envelope plus a payload or an error.
#[derive(Debug, Clone, PartialEq)]
pub struct SubsonicResponse {
    result: Result<Payload, ApiError>,
    format: Format,
    status: StatusCode,
}

impl SubsonicResponse {
    #[must_use]
    pub fn ok(format: Format, payload: impl Into<Payload>) -> Self {
        Self {
            result: Ok(payload.into()),
            format,
            status: StatusCode::OK,
        }
    }

    #[must_use]
    pub fn empty(format: Format) -> Self {
        Self::ok(format, Payload::default())
    }

    #[must_use]
    pub fn error(format: Format, error: ApiError) -> Self {
        Self {
            result: Err(error),
            format,
            status: StatusCode::OK,
        }
    }

    /// Overrides the HTTP status (Subsonic errors are normally sent as 200).
    #[must_use]
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    fn envelope(&self) -> Element {
        let status = if self.result.is_ok() { "ok" } else { "failed" };
        let mut root = Element::new("subsonic-response")
            .attr("status", status)
            .attr("version", API_VERSION)
            .attr("type", SERVER_TYPE)
            .attr("serverVersion", pixiu_core::VERSION)
            .attr("openSubsonic", true);
        match &self.result {
            Ok(payload) => root.children.extend(payload.0.iter().cloned()),
            Err(error) => {
                root = root.child(
                    Element::new("error")
                        .attr("code", error.code as u8)
                        .attr("message", error.message.as_str()),
                );
            }
        }
        root
    }

    #[must_use]
    pub fn to_xml(&self) -> String {
        let envelope = self.envelope();
        let mut writer = Writer::new(Vec::new());
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .expect("writing XML to a Vec cannot fail");

        let mut start = BytesStart::new(envelope.name);
        start.push_attribute(("xmlns", XML_NAMESPACE));
        for (key, value) in &envelope.attrs {
            start.push_attribute((*key, value.to_xml().as_str()));
        }
        write_xml_contents(&mut writer, start, envelope.name, &envelope.children, None);

        String::from_utf8(writer.into_inner()).expect("quick-xml writes UTF-8")
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        let envelope = self.envelope();
        let mut root = Map::new();
        root.insert(envelope.name.to_owned(), envelope.to_json());
        Json::Object(root).to_string()
    }
}

impl IntoResponse for SubsonicResponse {
    fn into_response(self) -> Response {
        let (content_type, body) = match &self.format {
            Format::Xml => ("text/xml; charset=utf-8", self.to_xml()),
            Format::Json => ("application/json", self.to_json()),
            Format::Jsonp(callback) => (
                "application/javascript",
                format!("{callback}({});", self.to_json()),
            ),
        };
        let mut response = (self.status, body).into_response();
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ok_envelope() {
        let response = SubsonicResponse::empty(Format::Xml);
        insta::assert_snapshot!(response.to_xml(), @r#"<?xml version="1.0" encoding="UTF-8"?><subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.16.1" type="pixiu" serverVersion="0.1.0" openSubsonic="true"/>"#);
        insta::assert_snapshot!(response.to_json(), @r#"{"subsonic-response":{"status":"ok","version":"1.16.1","type":"pixiu","serverVersion":"0.1.0","openSubsonic":true}}"#);
    }

    #[test]
    fn lists_are_arrays_in_json_and_repeated_in_xml() {
        let payload = Element::new("album")
            .attr("id", "al-1")
            .attr("songCount", 2_u32)
            .list(
                "song",
                [
                    Element::new("song").attr("id", "tr-1"),
                    Element::new("song").attr("id", "tr-2"),
                ],
            )
            .list("genres", [Element::new("genres").attr("name", "Rock")]);
        let response = SubsonicResponse::ok(Format::Json, payload);

        insta::assert_snapshot!(response.to_json(), @r#"{"subsonic-response":{"status":"ok","version":"1.16.1","type":"pixiu","serverVersion":"0.1.0","openSubsonic":true,"album":{"id":"al-1","songCount":2,"song":[{"id":"tr-1"},{"id":"tr-2"}],"genres":[{"name":"Rock"}]}}}"#);
        insta::assert_snapshot!(response.to_xml(), @r#"<?xml version="1.0" encoding="UTF-8"?><subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.16.1" type="pixiu" serverVersion="0.1.0" openSubsonic="true"><album id="al-1" songCount="2"><song id="tr-1"/><song id="tr-2"/><genres name="Rock"/></album></subsonic-response>"#);
    }

    #[test]
    fn top_level_lists_and_scalar_values() {
        let response = SubsonicResponse::ok(
            Format::Json,
            Payload::list(
                "openSubsonicExtensions",
                [Element::new("openSubsonicExtensions")
                    .attr("name", "formPost")
                    .values("versions", [1])],
            ),
        );

        insta::assert_snapshot!(response.to_json(), @r#"{"subsonic-response":{"status":"ok","version":"1.16.1","type":"pixiu","serverVersion":"0.1.0","openSubsonic":true,"openSubsonicExtensions":[{"name":"formPost","versions":[1]}]}}"#);
        insta::assert_snapshot!(response.to_xml(), @r#"<?xml version="1.0" encoding="UTF-8"?><subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.16.1" type="pixiu" serverVersion="0.1.0" openSubsonic="true"><openSubsonicExtensions name="formPost"><versions>1</versions></openSubsonicExtensions></subsonic-response>"#);
    }

    #[test]
    fn text_content_is_escaped_and_becomes_value() {
        let response = SubsonicResponse::ok(
            Format::Json,
            Element::new("lyrics")
                .attr("artist", "A & B")
                .text("la <la>"),
        );

        insta::assert_snapshot!(response.to_json(), @r#"{"subsonic-response":{"status":"ok","version":"1.16.1","type":"pixiu","serverVersion":"0.1.0","openSubsonic":true,"lyrics":{"artist":"A & B","value":"la <la>"}}}"#);
        insta::assert_snapshot!(response.to_xml(), @r#"<?xml version="1.0" encoding="UTF-8"?><subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.16.1" type="pixiu" serverVersion="0.1.0" openSubsonic="true"><lyrics artist="A &amp; B">la &lt;la&gt;</lyrics></subsonic-response>"#);
    }

    #[test]
    fn fields_are_child_elements_in_xml_and_scalars_in_json() {
        let response = SubsonicResponse::ok(
            Format::Json,
            Element::new("artistInfo2")
                .field("musicBrainzId", "mbid-1")
                .list("similarArtist", Vec::new()),
        );
        insta::assert_snapshot!(response.to_json(), @r#"{"subsonic-response":{"status":"ok","version":"1.16.1","type":"pixiu","serverVersion":"0.1.0","openSubsonic":true,"artistInfo2":{"musicBrainzId":"mbid-1","similarArtist":[]}}}"#);
        insta::assert_snapshot!(response.to_xml(), @r#"<?xml version="1.0" encoding="UTF-8"?><subsonic-response xmlns="http://subsonic.org/restapi" status="ok" version="1.16.1" type="pixiu" serverVersion="0.1.0" openSubsonic="true"><artistInfo2><musicBrainzId>mbid-1</musicBrainzId></artistInfo2></subsonic-response>"#);
    }

    #[test]
    fn failed_envelope() {
        let response = SubsonicResponse::error(
            Format::Xml,
            ApiError::new(ErrorCode::WrongCredentials, "Wrong username or password"),
        );
        insta::assert_snapshot!(response.to_xml(), @r#"<?xml version="1.0" encoding="UTF-8"?><subsonic-response xmlns="http://subsonic.org/restapi" status="failed" version="1.16.1" type="pixiu" serverVersion="0.1.0" openSubsonic="true"><error code="40" message="Wrong username or password"/></subsonic-response>"#);
        insta::assert_snapshot!(response.to_json(), @r#"{"subsonic-response":{"status":"failed","version":"1.16.1","type":"pixiu","serverVersion":"0.1.0","openSubsonic":true,"error":{"code":40,"message":"Wrong username or password"}}}"#);
    }
}
