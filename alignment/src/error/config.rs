#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Argument {0} is missing")]
    ArgumentNone(String)
}