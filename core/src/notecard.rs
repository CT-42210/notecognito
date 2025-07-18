use serde::{Deserialize, Serialize};
use crate::error::{NotecognitoError, Result};

/// Represents a notecard ID (1-9)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotecardId(u8);

impl NotecardId {
    /// Creates a new NotecardId, validating that it's between 1 and 9
    pub fn new(id: u8) -> Result<Self> {
        if id >= 1 && id <= 9 {
            Ok(NotecardId(id))
        } else {
            Err(NotecognitoError::InvalidNotecardId(id))
        }
    }

    /// Gets the inner u8 value
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for NotecardId {
    type Error = NotecognitoError;

    fn try_from(value: u8) -> Result<Self> {
        NotecardId::new(value)
    }
}

impl std::fmt::Display for NotecardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a single notecard with its content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notecard {
    /// The notecard's ID (1-9)
    pub id: NotecardId,
    /// The text content to display (supports multi-line)
    pub content: String,
}

impl Notecard {
    /// Creates a new notecard with the given ID and content
    pub fn new(id: NotecardId, content: String) -> Self {
        Notecard { id, content }
    }

    /// Creates an empty notecard with the given ID
    pub fn empty(id: NotecardId) -> Self {
        Notecard {
            id,
            content: String::new(),
        }
    }

    /// Validates the notecard content
    pub fn validate(&self) -> Result<()> {
        // Add any content validation rules here
        // For now, we'll just ensure the content isn't too long
        const MAX_CONTENT_LENGTH: usize = 10000;

        if self.content.len() > MAX_CONTENT_LENGTH {
            return Err(NotecognitoError::Config(
                format!("Notecard content exceeds maximum length of {} characters", MAX_CONTENT_LENGTH)
            ));
        }

        Ok(())
    }
}