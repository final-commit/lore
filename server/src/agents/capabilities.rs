use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// All capabilities an agent may request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadDocs,
    WriteDocs,
    ReadMessages,
    WriteMessages,
    HttpOutbound,
    ExecuteCode,
    BrowseWeb,
    ShellRun,
    CreateIssues,
    ReadUsers,
    ReadCode,
}

impl Capability {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read_docs" => Some(Self::ReadDocs),
            "write_docs" => Some(Self::WriteDocs),
            "read_messages" => Some(Self::ReadMessages),
            "write_messages" => Some(Self::WriteMessages),
            "http_outbound" => Some(Self::HttpOutbound),
            "execute_code" => Some(Self::ExecuteCode),
            "browse_web" => Some(Self::BrowseWeb),
            "shell_run" => Some(Self::ShellRun),
            "create_issues" => Some(Self::CreateIssues),
            "read_users" => Some(Self::ReadUsers),
            "read_code" => Some(Self::ReadCode),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadDocs => "read_docs",
            Self::WriteDocs => "write_docs",
            Self::ReadMessages => "read_messages",
            Self::WriteMessages => "write_messages",
            Self::HttpOutbound => "http_outbound",
            Self::ExecuteCode => "execute_code",
            Self::BrowseWeb => "browse_web",
            Self::ShellRun => "shell_run",
            Self::CreateIssues => "create_issues",
            Self::ReadUsers => "read_users",
            Self::ReadCode => "read_code",
        }
    }
}

/// A set of capabilities granted to an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    granted: HashSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            granted: HashSet::new(),
        }
    }

    /// Grant a capability.
    pub fn grant(&mut self, cap: Capability) {
        self.granted.insert(cap);
    }

    /// Check if a capability is granted.
    pub fn has(&self, cap: &Capability) -> bool {
        self.granted.contains(cap)
    }

    /// Build from a list of capability name strings.
    pub fn from_strings(caps: &[String]) -> Result<Self, AppError> {
        let mut set = Self::new();
        for s in caps {
            match Capability::from_str(s) {
                Some(c) => set.grant(c),
                None => {
                    return Err(AppError::BadRequest(format!(
                        "unknown capability: {s}"
                    )))
                }
            }
        }
        Ok(set)
    }

    pub fn as_string_vec(&self) -> Vec<String> {
        let mut v: Vec<String> = self.granted.iter().map(|c| c.as_str().to_string()).collect();
        v.sort();
        v
    }
}

/// Guards capability checks at call time.
pub struct CapabilityGuard {
    caps: CapabilitySet,
}

impl CapabilityGuard {
    pub fn new(caps: CapabilitySet) -> Self {
        Self { caps }
    }

    /// Check that the required capability is granted; return error if not.
    pub fn require(&self, cap: &Capability) -> Result<(), AppError> {
        if self.caps.has(cap) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "capability denied: {}",
                cap.as_str()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_set_grant_and_check() {
        let mut set = CapabilitySet::new();
        assert!(!set.has(&Capability::ReadDocs));
        set.grant(Capability::ReadDocs);
        assert!(set.has(&Capability::ReadDocs));
        assert!(!set.has(&Capability::WriteDocs));
    }

    #[test]
    fn test_capability_from_strings() {
        let caps =
            CapabilitySet::from_strings(&["read_docs".to_string(), "http_outbound".to_string()])
                .unwrap();
        assert!(caps.has(&Capability::ReadDocs));
        assert!(caps.has(&Capability::HttpOutbound));
        assert!(!caps.has(&Capability::ExecuteCode));
    }

    #[test]
    fn test_capability_from_strings_invalid() {
        let result = CapabilitySet::from_strings(&["nonexistent_cap".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_capability_guard_allow() {
        let mut set = CapabilitySet::new();
        set.grant(Capability::ReadDocs);
        let guard = CapabilityGuard::new(set);
        assert!(guard.require(&Capability::ReadDocs).is_ok());
    }

    #[test]
    fn test_capability_guard_deny() {
        let set = CapabilitySet::new();
        let guard = CapabilityGuard::new(set);
        let result = guard.require(&Capability::ReadDocs);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("capability denied"));
    }
}
