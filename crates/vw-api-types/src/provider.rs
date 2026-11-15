//! 模型提供商与模型列表契约的兼容重导出。

pub use vw_api_provider::{
    AuthKind, ConnectProviderRequest, DisconnectProviderRequest, GetProviderResponse,
    ListModelsResponse, ListProvidersRequest, ListProvidersResponse, ModelDto, PatchProviderModelDto,
    PatchProviderModelsRequest, ProviderDto, ProviderStatus, RefreshProvidersRequest,
    RefreshProvidersResponse, UpdateProviderRequest,
};
