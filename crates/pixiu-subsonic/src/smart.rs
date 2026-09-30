//! Smart playlists: songs chosen by rules instead of listed by hand, in
//! the web player's (koel's) model. Rules come in groups; a song belongs
//! when it matches every rule of any group. A smart playlist's songs are
//! worked out whenever it is read.

use jiff::{Timestamp, ToSpan};
use pixiu_db::{
    Library,
    owned::{Bind, Sql},
    toasty,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// The most songs a smart playlist lists.
const MAX_SONGS: i64 = 5000;

/// Rules that must all match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleGroup {
    #[serde(default)]
    pub id: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default)]
    pub id: String,
    /// What the rule looks at, e.g. `title` or `interactions.play_count`.
    pub model: String,
    /// How, e.g. `contains` or `inLast`.
    pub operator: String,
    /// One value, or two for `isBetween`.
    pub value: Vec<Json>,
}

/// Why rules cannot be used.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuleError {
    #[error("the rules are not valid JSON: {0}")]
    Json(String),
    #[error("a smart playlist needs at least one rule")]
    Empty,
    #[error("unknown field `{0}`")]
    Field(String),
    #[error("`{operator}` does not apply to {field}")]
    Operator { field: String, operator: String },
    #[error("{0} needs a value")]
    Value(String),
}

/// What kind of values a field holds, which decides its operators.
#[derive(Clone, Copy)]
enum Kind {
    Text,
    Number,
    Date,
}

/// A field's SQL expression and kind. Lengths are compared in seconds.
fn field(model: &str) -> Option<(&'static str, Kind)> {
    Some(match model {
        "title" => ("tracks.title", Kind::Text),
        "album.name" => ("albums.title", Kind::Text),
        "artist.name" => ("tracks.artist_credit", Kind::Text),
        "genre" => ("tracks.genre", Kind::Text),
        "year" => ("COALESCE(tracks.year, albums.year)", Kind::Number),
        "length" => ("(tracks.duration_ms / 1000)", Kind::Number),
        "interactions.play_count" => ("COALESCE(an.play_count, 0)", Kind::Number),
        "rating" => ("COALESCE(an.rating, 0)", Kind::Number),
        "interactions.last_played_at" => ("an.last_played", Kind::Date),
        "created_at" => ("tracks.added_at", Kind::Date),
        _ => return None,
    })
}

fn operators(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Text => &[
            "is",
            "isNot",
            "contains",
            "notContain",
            "beginsWith",
            "endsWith",
        ],
        Kind::Number => &["is", "isNot", "isGreaterThan", "isLessThan", "isBetween"],
        Kind::Date => &["is", "isNot", "inLast", "notInLast", "isBetween"],
    }
}

/// Reads and checks stored or submitted rules.
///
/// # Errors
///
/// Fails for malformed JSON, no rules at all, unknown fields, operators a
/// field does not take, and missing values.
pub fn parse(json: &str) -> Result<Vec<RuleGroup>, RuleError> {
    let groups: Vec<RuleGroup> =
        serde_json::from_str(json).map_err(|error| RuleError::Json(error.to_string()))?;
    let groups: Vec<RuleGroup> = groups
        .into_iter()
        .filter(|group| !group.rules.is_empty())
        .collect();
    if groups.is_empty() {
        return Err(RuleError::Empty);
    }
    for rule in groups.iter().flat_map(|group| &group.rules) {
        let (_, kind) = field(&rule.model).ok_or_else(|| RuleError::Field(rule.model.clone()))?;
        if !operators(kind).contains(&rule.operator.as_str()) {
            return Err(RuleError::Operator {
                field: rule.model.clone(),
                operator: rule.operator.clone(),
            });
        }
        let needed = if rule.operator == "isBetween" { 2 } else { 1 };
        let given = rule
            .value
            .iter()
            .filter(|value| !text(value).is_empty())
            .count();
        if given < needed {
            return Err(RuleError::Value(rule.model.clone()));
        }
    }
    Ok(groups)
}

/// A value as text, whether the player sent a string or a number.
fn text(value: &Json) -> String {
    match value {
        Json::String(text) => text.trim().to_owned(),
        Json::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

fn number(value: &Json) -> i64 {
    match value {
        Json::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|float| float as i64))
            .unwrap_or(0),
        other => text(other).parse::<f64>().map_or(0, |float| float as i64),
    }
}

/// `%` and `_` match themselves in `LIKE` patterns.
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Rule conditions, as SQL whose parameters `sql` binds.
struct Conditions<'a> {
    sql: &'a mut Sql,
}

impl Conditions<'_> {
    fn param(&mut self, bind: Bind) -> String {
        self.sql.param(bind)
    }

    fn condition(&mut self, rule: &Rule, now: Timestamp) -> String {
        let (column, kind) = field(&rule.model).expect("rules are parsed before use");
        let first = rule.value.first().unwrap_or(&Json::Null);
        let second = rule.value.get(1).unwrap_or(&Json::Null);
        match kind {
            Kind::Text => {
                let value = text(first);
                let (pattern, negated) = match rule.operator.as_str() {
                    "is" => (escape_like(&value), false),
                    "isNot" => (escape_like(&value), true),
                    "contains" => (format!("%{}%", escape_like(&value)), false),
                    "notContain" => (format!("%{}%", escape_like(&value)), true),
                    "beginsWith" => (format!("{}%", escape_like(&value)), false),
                    _ => (format!("%{}", escape_like(&value)), false),
                };
                let pattern = self.param(Bind::Text(pattern));
                // LIKE ignores case, as the player's filters do.
                if negated {
                    format!("({column} IS NULL OR {column} NOT LIKE {pattern} ESCAPE '\\')")
                } else {
                    format!("{column} LIKE {pattern} ESCAPE '\\'")
                }
            }
            Kind::Number => {
                let low = self.param(Bind::Int(number(first)));
                match rule.operator.as_str() {
                    "is" => format!("{column} = {low}"),
                    "isNot" => format!("{column} != {low}"),
                    "isGreaterThan" => format!("{column} > {low}"),
                    "isLessThan" => format!("{column} < {low}"),
                    _ => {
                        let high = self.param(Bind::Int(number(second)));
                        format!("{column} BETWEEN MIN({low}, {high}) AND MAX({low}, {high})")
                    }
                }
            }
            // Timestamps are stored as RFC 3339 in UTC, so their first ten
            // characters are the date, and they sort as text.
            Kind::Date => match rule.operator.as_str() {
                "is" => {
                    let day = self.param(Bind::Text(text(first)));
                    format!("substr({column}, 1, 10) = {day}")
                }
                "isNot" => {
                    let day = self.param(Bind::Text(text(first)));
                    format!("({column} IS NULL OR substr({column}, 1, 10) != {day})")
                }
                "inLast" | "notInLast" => {
                    let days = number(first).clamp(0, 365 * 200);
                    let cutoff = now
                        .checked_sub((days * 24).hours())
                        .unwrap_or(Timestamp::MIN)
                        .to_string();
                    let cutoff = self.param(Bind::Text(cutoff));
                    if rule.operator == "inLast" {
                        format!("{column} >= {cutoff}")
                    } else {
                        format!("({column} IS NULL OR {column} < {cutoff})")
                    }
                }
                _ => {
                    let (from, to) = (text(first), text(second));
                    let (from, to) = if from <= to { (from, to) } else { (to, from) };
                    let from = self.param(Bind::Text(from));
                    let to = self.param(Bind::Text(to));
                    format!("substr({column}, 1, 10) BETWEEN {from} AND {to}")
                }
            },
        }
    }
}

/// The SQL selecting the songs of `owner`'s library that `groups` match,
/// in album order.
fn query(owner: u64, groups: &[RuleGroup], now: Timestamp) -> Sql {
    let mut sql = Sql::owned(owner, "");
    let mut conditions = Conditions { sql: &mut sql };
    let any: Vec<String> = groups
        .iter()
        .map(|group| {
            let all: Vec<String> = group
                .rules
                .iter()
                .map(|rule| conditions.condition(rule, now))
                .collect();
            format!("({})", all.join(" AND "))
        })
        .collect();
    let limit = conditions.param(Bind::Int(MAX_SONGS));
    sql.push(&format!(
        "SELECT tracks.id FROM tracks \
         JOIN albums ON albums.id = tracks.album_id \
         LEFT JOIN annotations an ON an.item = 'tr-' || tracks.id \
         AND an.user_id = tracks.user_id \
         WHERE tracks.user_id = ?1 AND ({}) \
         ORDER BY albums.title_key, tracks.disc_number, tracks.track_number, tracks.id \
         LIMIT {limit}",
        any.join(" OR ")
    ));
    sql
}

/// The songs of the library a smart playlist's stored rules match now.
/// Rules that no longer parse match nothing.
///
/// # Errors
///
/// Fails on database errors.
pub async fn track_ids(lib: &Library, rules: &str) -> Result<Vec<u64>, toasty::Error> {
    let groups = match parse(rules) {
        Ok(groups) => groups,
        Err(error) => {
            tracing::warn!(%error, "ignoring a smart playlist's broken rules");
            return Ok(Vec::new());
        }
    };
    query(lib.owner(), &groups, pixiu_db::now())
        .ids(&mut lib.db())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(json: serde_json::Value) -> Result<Vec<RuleGroup>, RuleError> {
        parse(&json.to_string())
    }

    #[test]
    fn rules_are_checked() {
        let valid = json_rules(&[("title", "contains", &["light"])]);
        assert!(rules(valid).is_ok());
        assert_eq!(rules(serde_json::json!([])), Err(RuleError::Empty));
        assert_eq!(
            rules(json_rules(&[("owner", "is", &["me"])])),
            Err(RuleError::Field("owner".to_owned()))
        );
        assert!(matches!(
            rules(json_rules(&[("year", "contains", &["19"])])),
            Err(RuleError::Operator { .. })
        ));
        assert_eq!(
            rules(json_rules(&[("year", "isBetween", &["1990"])])),
            Err(RuleError::Value("year".to_owned()))
        );
        assert!(matches!(parse("not json"), Err(RuleError::Json(_))));
    }

    #[test]
    fn groups_are_or_rules_are_and() {
        let groups = rules(serde_json::json!([
            { "id": "a", "rules": [
                { "id": "1", "model": "title", "operator": "beginsWith", "value": ["50%"] },
                { "id": "2", "model": "interactions.play_count", "operator": "isGreaterThan", "value": [3] },
            ]},
            { "id": "b", "rules": [
                { "id": "3", "model": "created_at", "operator": "inLast", "value": ["7"] },
            ]},
        ]))
        .unwrap();
        let now: Timestamp = "2026-09-29T12:00:00Z".parse().unwrap();
        let sql = query(7, &groups, now);
        let (text, binds) = (sql.text(), sql.binds());
        assert!(
            text.contains(
                "WHERE tracks.user_id = ?1 AND ((tracks.title LIKE ?2 ESCAPE '\\' \
                 AND COALESCE(an.play_count, 0) > ?3) OR (tracks.added_at >= ?4))"
            ),
            "{text}"
        );
        assert_eq!(binds[0], Bind::Int(7));
        assert!(matches!(&binds[1], Bind::Text(pattern) if pattern == "50\\%%"));
        assert!(matches!(&binds[3], Bind::Text(cutoff) if cutoff == "2026-09-22T12:00:00Z"));
    }

    fn json_rules(rules: &[(&str, &str, &[&str])]) -> serde_json::Value {
        serde_json::json!([{
            "id": "g",
            "rules": rules.iter().map(|(model, operator, value)| serde_json::json!({
                "id": "r", "model": model, "operator": operator, "value": value,
            })).collect::<Vec<_>>(),
        }])
    }
}
