pub struct BindingFixtureProvider;

#[async_trait::async_trait]
impl model_types::Provider for BindingFixtureProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("unexpected transport")
    }
}
