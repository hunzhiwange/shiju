//! 模型 JSON 兼容性测试模块，验证 provider model 结构可接受对象形式的实验元数据。

use vw_shared::provider::models::Provider;

#[test]
fn provider_model_allows_object_experimental_metadata() {
    let json = r#"
    {
      "id": "openai",
      "name": "OpenAI",
      "models": {
        "gpt-5": {
          "id": "gpt-5",
          "name": "GPT-5",
          "experimental": {
            "modes": {
              "fast": {
                "provider": {
                  "body": {
                    "service_tier": "priority"
                  }
                }
              }
            }
          }
        }
      }
    }
    "#;

    let provider: Provider = serde_json::from_str(json).expect("provider json should deserialize");
    let model = provider.models.get("gpt-5").expect("model should exist");

    assert!(model.experimental.is_some());
    assert!(model.experimental.as_ref().and_then(|value| value.get("modes")).is_some());
}
