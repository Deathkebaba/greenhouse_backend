use greenhouse_macro::IntoJsonResponse;
use serde::{Deserialize, Serialize};

use super::image_metadata::DiaryImageMetadataDto;

#[derive(Serialize, Deserialize, Debug, IntoJsonResponse)]
pub struct DiaryEntryResponseDto {
    pub id: String,
    pub date: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub images: Vec<DiaryImageMetadataDto>,
    pub created_at: String,
    pub updated_at: String,
}
