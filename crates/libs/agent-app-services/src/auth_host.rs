pub struct AuthAccountHost {
    pub id: String,
    pub home: std::path::PathBuf,
    pub credential_root: std::path::PathBuf,
    pub principal: String,
    pub owns_native_slot: bool,
    pub can_set_default: bool,
    pub can_revoke: bool,
}
