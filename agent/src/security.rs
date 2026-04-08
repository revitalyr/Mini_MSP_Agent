use std::path::Path;

#[derive(Clone, Debug)]
pub struct SecurityPolicy {
    pub allowed_commands: Vec<String>,
    pub max_file_size: u64,
}

impl SecurityPolicy {
    pub fn new(allowed_commands: Vec<String>, max_file_size: u64) -> Self {
        Self { allowed_commands, max_file_size }
    }

    pub fn is_command_allowed(&self, cmd: &str) -> bool {
        let forbidden_chars = ['|', ';', '&', '$', '>', '<', '`', '\\', '!', '(', ')', '\n'];
        if cmd.contains("..") || cmd.chars().any(|c| forbidden_chars.contains(&c)) {
            return false;
        }
        
        cmd.split_whitespace()
            .next()
            .map_or(false, |first| self.allowed_commands.iter().any(|c| c == first))
    }

    pub fn is_file_size_allowed(&self, path: impl AsRef<Path>) -> bool {
        match std::fs::metadata(path) {
            Ok(meta) => meta.len() <= self.max_file_size,
            Err(_) => false,
        }
    }
}