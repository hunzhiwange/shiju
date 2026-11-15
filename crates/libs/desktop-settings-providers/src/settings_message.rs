#[derive(Debug, Clone)]
pub enum ModelRoutesMessage {
    SaveDue(u64),
    Saved(u64, Result<(), String>),
    Loaded(
        u64,
        Result<
            (
                Vec<config_routing::ModelRouteConfig>,
                config_routing::QueryClassificationConfig,
            ),
            String,
        >,
    ),
    Refresh,
    AddRoute,
    RemoveRoute(usize),
    PatternChanged(usize, String),
    ProviderChanged(usize, String),
    ModelChanged(usize, String),
    PriorityChanged(usize, String),
}

#[derive(Debug, Clone, Copy)]
pub enum ModelConfigTabMessage { Selected(crate::state::ModelConfigTab) }
