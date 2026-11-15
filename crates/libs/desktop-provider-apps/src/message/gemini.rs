#[derive(Clone)]
pub enum Field {
    ThirdParty(bool),
    BaseUrl(String),
    Model(String),
    Apply(bool),
}
