//! Users, roles, and the household.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What a user is allowed to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// Full control, including managing users.
    Owner,
    /// May view and modify household data.
    Editor,
    /// Read-only.
    Viewer,
}

impl UserRole {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            UserRole::Owner => "owner",
            UserRole::Editor => "editor",
            UserRole::Viewer => "viewer",
        }
    }

    /// Whether the role may create or change data.
    pub fn can_write(self) -> bool {
        !matches!(self, UserRole::Viewer)
    }

    /// Whether the role may manage users and see the whole household.
    pub fn is_owner(self) -> bool {
        matches!(self, UserRole::Owner)
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`UserRole`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid role: {0}")]
pub struct UserRoleParseError(pub String);

impl FromStr for UserRole {
    type Err = UserRoleParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "owner" => Ok(UserRole::Owner),
            "editor" => Ok(UserRole::Editor),
            "viewer" => Ok(UserRole::Viewer),
            other => Err(UserRoleParseError(other.to_string())),
        }
    }
}

/// A household member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Surrogate identifier.
    pub id: Uuid,
    /// Login name.
    pub username: String,
    /// Name shown in the UI.
    pub display_name: String,
    /// Permissions.
    pub role: UserRole,
}

/// The fields required to create a user.
#[derive(Debug, Clone)]
pub struct NewUser {
    /// Login name.
    pub username: String,
    /// Name shown in the UI.
    pub display_name: String,
    /// Argon2 password hash.
    pub password_hash: String,
    /// Permissions.
    pub role: UserRole,
}

impl User {
    /// Whether this user may see an account with the given ownership.
    pub fn can_see(&self, ownership: crate::accounts::OwnershipScope, owner: Option<Uuid>) -> bool {
        self.role.is_owner()
            || ownership == crate::accounts::OwnershipScope::Household
            || owner == Some(self.id)
    }

    /// Whether this user may modify an account with the given ownership.
    pub fn can_modify(
        &self,
        ownership: crate::accounts::OwnershipScope,
        owner: Option<Uuid>,
    ) -> bool {
        if !self.role.can_write() {
            return false;
        }
        self.role.is_owner()
            || ownership == crate::accounts::OwnershipScope::Household
            || owner == Some(self.id)
    }
}
