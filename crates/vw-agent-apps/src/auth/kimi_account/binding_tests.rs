use super::*;

#[tokio::test]
async fn provider_kimi_binding_consumer_rechecks_native_identity() {
    let f = Fixture::new();
    let target = f.target(AuthRegion::Global);
    f.credentials
        .lease_kimi(target.scope, ACCOUNT, false)
        .unwrap()
        .save(&AccountCredential::KimiOauth(KimiCredential {
            access_token: "fixture-current-access".into(),
            refresh_token: "fixture-current-refresh".into(),
            expires_at: TimestampMs(FUTURE * 1_000),
            device_id: DEVICE.into(),
            region: AuthRegion::Global,
        }))
        .unwrap();
    f.write_native(&native_tokens("fixture-current", FUTURE));
    let connection =
        &crate::providers::kimi::declarations(Some(CONFIG.as_bytes())).unwrap()["subscription"]
            .declaration["provider"];

    assert_eq!(
        f.native()
            .bind_account(&f.credentials, f.target(AuthRegion::Global), None, connection, || false)
            .await,
        Err(AuthErrorCode::Conflict),
    );
    assert_eq!(fs::read_to_string(f.config()).unwrap(), CONFIG);
    assert_eq!(f.native().bound_account(target.scope, connection), Ok(None));
    f.write_native(&native_tokens("fixture-independent", FUTURE));
    for (identity, result) in [(99, Err(AuthErrorCode::IdentityMismatch)), (41, Ok(()))] {
        let (url, request) = oauth_server("200 OK", "", json!({"user_id":identity})).await;
        assert_eq!(
            f.native()
                .bind_account_with(
                    &f.credentials,
                    f.target(AuthRegion::Global),
                    None,
                    connection,
                    || true,
                    |request| {
                        assert_eq!(request.url().as_str(), "https://api.kimi.ai/coding/v1/me");
                        *request.url_mut() = url.clone();
                    },
                )
                .await,
            result,
        );
        assert!(request.await.unwrap().contains("Bearer fixture-independent-access"));
        if result.is_err() {
            assert_eq!(f.native().bound_account(target.scope, connection), Ok(None));
            assert_eq!(fs::read_to_string(f.config()).unwrap(), CONFIG);
        }
    }
    assert_eq!(f.native().bound_account(target.scope, connection), Ok(Some(ACCOUNT.into())));
    // The user's CLI logged in again and changed its model independently.
    fs::write(f.config(), CONFIG).unwrap();
    f.write_native(&native_tokens("fixture-relogin", FUTURE));
    let auth = KimiDeviceAuth::new().unwrap();
    for (identity, result) in [(99, Err(AuthErrorCode::IdentityMismatch)), (41, Ok(()))] {
        let (url, request) = oauth_server("200 OK", "", json!({"user_id":identity})).await;
        assert_eq!(
            auth.use_account_with(
                &f.credentials,
                f.target(AuthRegion::Global),
                f.native(),
                |request| {
                    assert_eq!(request.url().as_str(), "https://api.kimi.ai/coding/v1/me");
                    *request.url_mut() = url.clone();
                },
            )
            .await,
            result,
        );
        assert!(request.await.unwrap().contains("Bearer fixture-relogin-access"));
        let current = fs::read_to_string(f.config()).unwrap();
        if result.is_err() {
            assert_eq!(current, CONFIG);
        } else {
            assert_eq!(current, CONFIG.replacen("\"original\" #", "\"chosen\" #", 1));
        }
    }
}
