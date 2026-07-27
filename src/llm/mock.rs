//! Mock provider that returns a hardcoded commit message.
//! Used for testing the full workflow without a real LLM.

use anyhow::Result;
use std::sync::Mutex;

pub struct MockProvider {
    responses: Vec<String>,
    index: Mutex<usize>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            responses: vec![r#"{"type":"feat","scope":"test","description":"add mock implementation","body":"Uses MockProvider to enable end-to-end testing without a real LLM"}"#.into()],
            index: Mutex::new(0),
        }
    }

    /// Create a provider that cycles through the given responses.
    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses,
            index: Mutex::new(0),
        }
    }

    pub async fn chat(&self, _system: &str, _user: &str) -> Result<String> {
        let resp = {
            let mut idx = self.index.lock().unwrap();
            let r = self.responses[*idx % self.responses.len()].clone();
            *idx += 1;
            r
        };
        Ok(resp)
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}
