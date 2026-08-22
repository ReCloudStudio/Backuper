#[derive(Debug, thiserror::Error)]
pub enum BackuperError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("配置解析错误: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("来源错误: {0}")]
    Source(String),

    #[error("存储错误: {0}")]
    Storage(String),

    #[error("规则不存在: {0}")]
    RuleNotFound(String),

    #[error("未知错误")]
    Unknown,
}
