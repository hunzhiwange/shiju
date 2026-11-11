use vw_gateway_client::vw_api_types::app_providers::desktop_mapping::*;
#[derive(Clone)]
pub(crate) struct Form {
    pub aggregation: Option<crate::app::message::settings::aggregation_desktop::Session>,
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
    pub fn new(id: String) -> Self {
        Self {
            aggregation: None,
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
