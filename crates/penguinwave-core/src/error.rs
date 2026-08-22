//! Core failure taxonomy.

use penguinwave_hid::HidError;
use penguinwave_proto::{ErrorKind, PwError};

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// A backend or HID failure that already carries its own classification.
    #[error("{0}")]
    Wire(#[from] PwError),

    #[error("preset {0:?} not found")]
    PresetNotFound(String),

    #[error("user device {0:?} not found")]
    NotFoundUserDevice(String),

    #[error("preset {0:?} is built in and cannot be modified")]
    BuiltinPreset(String),

    #[error("node {0:?} not found in the graph")]
    NodeNotFound(String),

    #[error("{0}")]
    Invalid(String),

    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{0}")]
    Internal(String),
}

impl From<HidError> for CoreError {
    fn from(e: HidError) -> Self {
        CoreError::Wire(e.into())
    }
}

impl From<CoreError> for PwError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::Wire(w) => w,
            CoreError::PresetNotFound(_)
            | CoreError::NotFoundUserDevice(_)
            | CoreError::NodeNotFound(_) => PwError::new(ErrorKind::NotFound, e.to_string()),
            CoreError::BuiltinPreset(_) | CoreError::Invalid(_) => {
                PwError::new(ErrorKind::BadRequest, e.to_string())
            }
            CoreError::Io { .. } | CoreError::Internal(_) => {
                PwError::new(ErrorKind::Internal, e.to_string())
            }
        }
    }
}

pub(crate) fn io_err(path: &std::path::Path, source: std::io::Error) -> CoreError {
    CoreError::Io {
        path: path.display().to_string(),
        source,
    }
}
