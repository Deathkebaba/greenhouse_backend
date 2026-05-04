use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DiaryImageMetadataDto {
    pub id: String,
    pub file_name: String,
    pub media_type: String,
    pub byte_size: i64,
    pub uploaded_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub download_url: String,
}
