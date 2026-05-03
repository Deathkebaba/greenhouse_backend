use greenhouse_macro::IntoJsonResponse;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::get_diary_entry::DiaryEntryResponseDto;
use super::query::DiaryTagFilterModeDto;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GetDiaryEntriesQueryDto {
    pub start: String,
    pub end: String,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_tags_as_json",
        deserialize_with = "deserialize_tags_from_json"
    )]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tag_filter_mode: DiaryTagFilterModeDto,
}

#[derive(Serialize, Deserialize, Debug, IntoJsonResponse)]
pub struct GetDiaryResponseDto {
    pub entries: Vec<DiaryEntryResponseDto>,
}

fn serialize_tags_as_json<S>(tags: &[String], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &serde_json::to_string(tags).map_err(serde::ser::Error::custom)?,
    )
}

fn deserialize_tags_from_json<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_tags = String::deserialize(deserializer)?;

    serde_json::from_str(&raw_tags).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_date_range_and_tag_filters_as_one_query_contract() {
        let query = GetDiaryEntriesQueryDto {
            start: String::from("2026-05-01T00:00:00Z"),
            end: String::from("2026-05-02T00:00:00Z"),
            tags: vec![String::from("Tomatoes"), String::from("Harvest")],
            tag_filter_mode: DiaryTagFilterModeDto::All,
        };

        let request = reqwest::Client::new()
            .get("http://example.test/diary")
            .query(&query)
            .build()
            .unwrap();
        let serialized = request.url().query().unwrap();

        assert!(serialized.contains("start=2026-05-01T00%3A00%3A00Z"));
        assert!(serialized.contains("end=2026-05-02T00%3A00%3A00Z"));
        assert!(serialized.contains("tags=%5B%22Tomatoes%22%2C%22Harvest%22%5D"));
        assert!(serialized.contains("tag_filter_mode=all"));
    }

    #[test]
    fn deserializes_json_encoded_tags_from_query_contract() {
        let query = "start=2026-05-01T00%3A00%3A00Z&end=2026-05-02T00%3A00%3A00Z&tags=%5B%22Tomatoes%22%2C%22Harvest%22%5D&tag_filter_mode=all";

        let decoded = serde_urlencoded::from_str::<GetDiaryEntriesQueryDto>(query).unwrap();

        assert_eq!(decoded.start, "2026-05-01T00:00:00Z");
        assert_eq!(decoded.end, "2026-05-02T00:00:00Z");
        assert_eq!(
            decoded.tags,
            vec![String::from("Tomatoes"), String::from("Harvest")]
        );
        assert_eq!(decoded.tag_filter_mode, DiaryTagFilterModeDto::All);
    }
}
