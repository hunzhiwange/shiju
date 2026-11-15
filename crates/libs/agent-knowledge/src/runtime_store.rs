//! Build the tenant store from the caller's selected configuration and home.
use agent_config::runtime::Config;
use agent_database::GlobalDatabase;
use gateway_error::ApiError;
use vw_api_transport::TenantScope;
use crate::store::SqliteKnowledgeStore;

pub fn knowledge_store_from_config(config: Config, home: &std::path::Path, tenant: TenantScope) -> Result<SqliteKnowledgeStore, ApiError> {
    let (embedder, resolved_embedding) = agent_memory::create_embedding_provider_with_routes(
        &config.memory,
        &config.embedding_routes,
        config.api_key.as_deref(),
    );
    let embedding_model = (embedder.dimensions() > 0).then_some(resolved_embedding.model);
    let database = GlobalDatabase::for_home(home).map_err(|error| {
        ApiError::internal(format!("initialize global knowledge database failed: {error}"))
    })?;
    SqliteKnowledgeStore::with_embedder(
        database,
        tenant,
        embedder,
        embedding_model,
        config.memory.vector_weight as f32,
        config.memory.keyword_weight as f32,
        config.memory.embedding_cache_size,
    )
}
