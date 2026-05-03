use crate::database;
use axum::http::StatusCode;
use derive_more::From;
use greenhouse_core::{
    http_error::{HttpErrorMapping, HttpErrorResponse},
    impl_http_error_from,
};
use serde::Serialize;

pub(crate) type HttpResult<T> = core::result::Result<T, HttpErrorResponse<Error>>;

#[derive(Debug, Serialize, From)]
pub(crate) enum Error {
    TimeError,
    InvalidRequest(String),
    #[from]
    Database(database::Error),
}

// region:    --- Error Boilerplate
impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

impl_http_error_from!(Error {
    crate::database::Error,
});

impl HttpErrorMapping for Error {
    fn to_status_code(&self) -> StatusCode {
        match self {
            Error::TimeError => StatusCode::BAD_REQUEST,
            Error::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Error::Database(e) => match e {
                database::Error::Creation => StatusCode::INTERNAL_SERVER_ERROR,
                database::Error::DatabaseConnection => StatusCode::INTERNAL_SERVER_ERROR,
                database::Error::Find => StatusCode::NOT_FOUND,
            },
        }
    }

    fn to_error_message(&self) -> String {
        match self {
            Error::TimeError => String::from("Time error"),
            Error::InvalidRequest(message) => message.clone(),
            Error::Database(e) => match e {
                database::Error::Creation => String::from("Database creation error"),
                database::Error::DatabaseConnection => String::from("Database connection error"),
                database::Error::Find => String::from("Database find error"),
            },
        }
    }
}

// endregion: --- Error Boilerplate
