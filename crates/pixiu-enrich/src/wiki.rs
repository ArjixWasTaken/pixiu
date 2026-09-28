//! Artist biographies and pictures, from the English Wikipedia article
//! Wikidata links to the artist's MusicBrainz entry.

use reqwest::{StatusCode, Url};
use serde::Deserialize;

use crate::EnrichError;

/// What Wikipedia says about an artist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistInfo {
    /// The article's summary, plain text.
    pub bio: String,
    /// The article, for attribution.
    pub url: String,
    pub image_url: Option<String>,
}

#[derive(Deserialize)]
struct EntityJson {
    entities: std::collections::HashMap<String, ItemJson>,
}

#[derive(Deserialize)]
struct ItemJson {
    #[serde(default)]
    sitelinks: std::collections::HashMap<String, SitelinkJson>,
}

#[derive(Deserialize)]
struct SitelinkJson {
    title: String,
}

#[derive(Deserialize)]
struct SummaryJson {
    #[serde(default)]
    extract: String,
    #[serde(rename = "type", default)]
    kind: String,
    originalimage: Option<ImageJson>,
    thumbnail: Option<ImageJson>,
    content_urls: Option<ContentUrlsJson>,
}

#[derive(Deserialize)]
struct ImageJson {
    source: String,
}

#[derive(Deserialize)]
struct ContentUrlsJson {
    desktop: PageJson,
}

#[derive(Deserialize)]
struct PageJson {
    page: String,
}

/// The article about a Wikidata item, summarized.
pub(crate) async fn artist_info(
    http: &reqwest::Client,
    wikidata_id: &str,
) -> Result<Option<ArtistInfo>, EnrichError> {
    let url = Url::parse(&format!(
        "https://www.wikidata.org/wiki/Special:EntityData/{wikidata_id}.json"
    ))
    .map_err(|error| EnrichError::Unexpected(error.to_string()))?;
    let entity: EntityJson = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let Some(title) = entity
        .entities
        .into_values()
        .find_map(|item| item.sitelinks.get("enwiki").map(|link| link.title.clone()))
    else {
        return Ok(None);
    };

    let mut url =
        Url::parse("https://en.wikipedia.org/api/rest_v1/page/summary/").expect("a valid base");
    url.path_segments_mut()
        .expect("a base with a path")
        .pop_if_empty()
        .push(&title.replace(' ', "_"));
    let response = http.get(url).send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let summary: SummaryJson = response.error_for_status()?.json().await?;
    // Disambiguation pages describe nobody in particular.
    if summary.extract.trim().is_empty() || summary.kind == "disambiguation" {
        return Ok(None);
    }
    Ok(Some(ArtistInfo {
        bio: summary.extract.trim().to_owned(),
        url: summary.content_urls.map_or_else(
            || format!("https://en.wikipedia.org/wiki/{}", title.replace(' ', "_")),
            |urls| urls.desktop.page,
        ),
        image_url: summary
            .thumbnail
            .or(summary.originalimage)
            .map(|image| image.source),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_are_read() {
        let summary: SummaryJson = serde_json::from_str(
            r#"{
                "type": "standard",
                "extract": "Kevin MacLeod is an American composer.",
                "thumbnail": {"source": "https://upload.wikimedia.org/thumb.jpg"},
                "content_urls": {"desktop": {"page": "https://en.wikipedia.org/wiki/Kevin_MacLeod"}}
            }"#,
        )
        .unwrap();
        assert_eq!(summary.extract, "Kevin MacLeod is an American composer.");
        assert_eq!(
            summary.thumbnail.unwrap().source,
            "https://upload.wikimedia.org/thumb.jpg"
        );
        let entity: EntityJson = serde_json::from_str(
            r#"{"entities": {"Q1": {"sitelinks": {"enwiki": {"title": "Kevin MacLeod"}}}}}"#,
        )
        .unwrap();
        assert_eq!(
            entity.entities["Q1"].sitelinks["enwiki"].title,
            "Kevin MacLeod"
        );
    }
}
