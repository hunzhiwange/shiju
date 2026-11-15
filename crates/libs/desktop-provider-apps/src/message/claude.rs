#[derive(Clone)]
pub enum Field {
    ThirdParty(bool),
    BaseUrl(String),
    ApiKeyHeader(bool),
    Model(String),
    Apply(bool),
}
