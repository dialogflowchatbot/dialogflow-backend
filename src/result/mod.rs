use serde::ser::{Serialize, SerializeStruct};
use std::convert::From;

pub(crate) type Result<D> = core::result::Result<D, Error>;

#[derive(Debug)]
pub(crate) enum Error {
    DbError(redb::Error),
    SerdeError(serde_json::Error),
    TimeFormatError(time::error::Format),
    ErrorWithMessage(String),
    NetworkConnectTimeout(reqwest::Error),
    NetworkReadTimeout(reqwest::Error),
    InvalidJsonStructure(serde_json::Error),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let message = match &self {
            Self::DbError(e) => format!("{:?}", e),
            Self::SerdeError(e) => format!("{:?}", e),
            Self::TimeFormatError(e) => format!("{:?}", e),
            Self::ErrorWithMessage(s) => String::from(s),
            Self::NetworkConnectTimeout(e) => format!("Network connect timeout: {:?}", e),
            Self::NetworkReadTimeout(e) => format!("Network read timeout: {:?}", e),
            Self::InvalidJsonStructure(e) => format!("Invalid JSON structure: {:?}", e),
        };
        let mut s = serializer.serialize_struct("Error", 1)?;
        s.serialize_field("message", &message)?;
        s.end()
    }
}

impl From<std::time::SystemTimeError> for Error {
    fn from(err: std::time::SystemTimeError) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}
impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<redb::Error> for Error {
    fn from(err: redb::Error) -> Self {
        Error::DbError(err)
    }
}

impl From<redb::TransactionError> for Error {
    fn from(err: redb::TransactionError) -> Self {
        Error::DbError(err.into())
    }
}

impl From<redb::DatabaseError> for Error {
    fn from(err: redb::DatabaseError) -> Self {
        Error::DbError(err.into())
    }
}

impl From<redb::StorageError> for Error {
    fn from(err: redb::StorageError) -> Self {
        Error::DbError(err.into())
    }
}

impl From<redb::TableError> for Error {
    fn from(err: redb::TableError) -> Self {
        Error::DbError(err.into())
    }
}

impl From<redb::CommitError> for Error {
    fn from(err: redb::CommitError) -> Self {
        Error::DbError(err.into())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeError(err)
    }
}

impl From<lettre::address::AddressError> for Error {
    fn from(err: lettre::address::AddressError) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<lettre::transport::smtp::Error> for Error {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<lettre::error::Error> for Error {
    fn from(err: lettre::error::Error) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<oasysdb::prelude::Error> for Error {
    fn from(err: oasysdb::prelude::Error) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<hf_hub::api::tokio::ApiError> for Error {
    fn from(err: hf_hub::api::tokio::ApiError) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Self {
        Error::ErrorWithMessage(format!("{:?}", err))
    }
}
