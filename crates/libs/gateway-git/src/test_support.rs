use super::*;
struct TestHost;
impl Host for TestHost {
    fn list_directories(&self, directory: String) -> Pin<Box<dyn Future<Output = Result<Vec<String>, ApiError>> + Send + '_>> {
        Box::pin(async move {
            agent_project::instance::provide(std::path::PathBuf::from(directory), None, || Box::pin(async {
                worktree_core::list_directories().await.map_err(|error| ApiError::bad_request(error.to_string()))
            })).await.map_err(|error| ApiError::bad_request(error.to_string()))?
        })
    }
}
pub fn host() -> WorktreeHost { WorktreeHost(Arc::new(TestHost)) }
