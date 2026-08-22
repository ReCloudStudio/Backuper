use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub id: String,
    pub schedule: String,
}
