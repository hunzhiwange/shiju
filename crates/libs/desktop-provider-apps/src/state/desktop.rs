use vw_api_app_provider_profiles::desktop_mapping::*;

pub(crate) const ACCOUNT_ID_REQUIRED: &str = "请在 API 地址中填入实际账户 ID，再保存供应商。";

pub(crate) fn needs_account_id(endpoint: &str) -> bool {
    endpoint.to_ascii_lowercase().contains("your_account_id")
}
#[derive(Clone)]
pub struct Form {
    pub id: String,
    pub mapping: DesktopMapping,
    pub new_route: String,
    pub busy: bool,
    pub notice: Option<String>,
    pub name: String,
    pub base_url: String,
    pub key: String,
    pub official: bool,
    pub create: bool,
    pub direct_editable: bool,
    pub mapping_editable: bool,
    pub apply: bool,
}
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DesktopForm([redacted])")
    }
}
impl Form {
    pub fn endpoint_needs_account_id(&self) -> bool {
        self.create && !self.official && needs_account_id(&self.base_url)
    }

    pub fn new(id: String) -> Self {
        Self {
            id,
            new_route: String::new(),
            mapping: DesktopMapping {
                mode: DesktopMappingMode::Direct,
                routes: Default::default(),
            },
            busy: false,
            notice: None,
            name: String::new(),
            base_url: String::new(),
            key: String::new(),
            official: false,
            create: true,
            direct_editable: true,
            mapping_editable: false,
            apply: false,
        }
    }
    pub fn load(&mut self, view: DesktopFormView) {
        self.name = view.name;
        self.base_url = view.base_url;
        self.official = view.official;
        self.mapping = view.mapping;
        self.direct_editable = view.direct_editable;
        self.mapping_editable = view.mapping_editable;
        self.create = false;
        self.busy = false;
    }
    pub fn finish(&mut self, result: Result<String, String>) {
        self.busy = false;
        self.notice = Some(match result {
            Ok(value) | Err(value) => value,
        });
    }
}
