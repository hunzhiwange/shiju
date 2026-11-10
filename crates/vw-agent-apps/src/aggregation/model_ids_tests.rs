use super::model_ids::{Decoded, ModelIdError, decode, encode};
use vw_api_types::agent_apps::AppId::{Claude, ClaudeDesktop, Codex, Pi};

#[test]
fn aggregation_t04_model_ids_preserves_contract() {
    for (one_m, expected) in [
        (false, "ccs-claude-relay-2--vendor/model--preview"),
        (true, "ccs-claude-relay-2--vendor/model--preview[1M]"),
    ] {
        let id = encode(Claude, "relay-2", "vendor/model--preview", one_m).unwrap();
        assert_eq!(id, expected);
        assert_eq!(
            decode(Claude, &id),
            Ok(Decoded::Stack {
                key: "relay-2",
                model: "vendor/model--preview",
                one_m,
            })
        );
    }
    assert_eq!(
        decode(Claude, "ccs-claude-relay--模型 [1m] "),
        Ok(Decoded::Stack {
            key: "relay",
            model: "模型",
            one_m: true,
        })
    );

    let id = encode(Codex, "relay", "vendor/model--preview", true).unwrap();
    assert_eq!(id, "ccs-relay/vendor/model--preview");
    assert_eq!(
        decode(Codex, &id),
        Ok(Decoded::Stack {
            key: "relay",
            model: "vendor/model--preview",
            one_m: false,
        })
    );
}

#[test]
fn aggregation_t04_desktop_codes_preserve_stable_digits() {
    for (model, digits) in [("model-6", "056772"), ("厂商/model", "366505")] {
        let id = encode(ClaudeDesktop, "relay", model, true).unwrap();
        assert_eq!(id, format!("claude-fable-ccs-{digits}"));
        assert_eq!(decode(ClaudeDesktop, &id), Ok(Decoded::Code(digits)));
        assert_eq!(
            decode(ClaudeDesktop, &format!("{id}[1m]")),
            Ok(Decoded::Code(digits))
        );
    }
}

#[test]
fn aggregation_t04_malformed_reserved_ids_are_rejected() {
    for (app, id) in [
        (Claude, "ccs-claude-relay"),
        (Claude, "ccs-claude---model"),
        (Claude, "ccs-claude-relay--[1M]"),
        (Claude, "ccs-claude-a/b--model"),
        (Codex, "ccs-relay"),
        (Codex, "ccs-"),
        (Codex, "ccs-/model"),
        (Codex, "ccs-relay/"),
        (Codex, "ccs-relay/ "),
        (Codex, "ccs-a--b/model"),
        (ClaudeDesktop, "claude-fable-ccs-12345"),
        (ClaudeDesktop, "claude-fable-ccs-12345678"),
        (ClaudeDesktop, "claude-fable-ccs-１２３４５６"),
        (ClaudeDesktop, "claude-fable-ccs-12345a[1m]"),
    ] {
        assert_eq!(decode(app, id), Err(ModelIdError::InvalidModelId), "{id}");
    }
}

#[test]
fn aggregation_t04_plain_ids_and_unsupported_apps_stay_distinct() {
    assert_eq!(decode(Claude, "claude-sonnet-5"), Ok(Decoded::Plain));
    assert_eq!(decode(Codex, "vendor/model"), Ok(Decoded::Plain));
    assert_eq!(
        decode(ClaudeDesktop, "claude-sonnet-5"),
        Ok(Decoded::Plain)
    );
    assert_eq!(
        decode(Pi, "ccs-relay/model"),
        Err(ModelIdError::UnsupportedApplication)
    );
    assert_eq!(
        encode(Pi, "relay", "model", false),
        Err(ModelIdError::UnsupportedApplication)
    );
    assert_eq!(
        encode(Claude, "a--b", "model", false),
        Err(ModelIdError::InvalidModelId)
    );
    assert_eq!(
        encode(Codex, "relay", "", false),
        Err(ModelIdError::InvalidModelId)
    );
}
