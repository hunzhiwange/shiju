use serde_json::json;

#[tokio::test]
async fn codex_plugin_compat_t37_denial_wins_and_recursion_fails_closed() {
    struct Verdict(bool);
    #[async_trait::async_trait]
    impl crate::HookHandler for Verdict {
        fn name(&self) -> &str {
            "fixture-verdict"
        }
        async fn permission_request(
            &self,
            _: &str,
            _: &serde_json::Value,
        ) -> anyhow::Result<Option<bool>> {
            Ok(Some(self.0))
        }
    }
    let mut runner = super::HookRunner::new();
    runner.register(Box::new(Verdict(true)));
    runner.register(Box::new(Verdict(false)));
    assert_eq!(runner.run_permission_request("marker", &json!({})).await.unwrap(), Some(false));
    struct Recursive;
    #[async_trait::async_trait]
    impl crate::HookHandler for Recursive {
        fn name(&self) -> &str {
            "fixture-recursive"
        }
        async fn permission_request(
            &self,
            name: &str,
            input: &serde_json::Value,
        ) -> anyhow::Result<Option<bool>> {
            super::HookRunner::new().run_permission_request(name, input).await
        }
    }
    let mut runner = super::HookRunner::new();
    runner.register(Box::new(Recursive));
    assert!(
        runner
            .run_permission_request("marker", &json!({}))
            .await
            .unwrap_err()
            .to_string()
            .contains("recursive")
    );
}
