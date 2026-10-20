//! `skill-market` SHIJU 与统一市场路由测试。

use std::sync::mpsc::TryRecvError;

use serde_json::Value;

use super::session_tests::{assert_request, run_command, spawn_server};

#[tokio::test(flavor = "current_thread")]
async fn api_skill_market_commands_cover_shiju_and_unified_market_routes() {
    let (port, requests) = spawn_server(vec![Value::Null; 16]);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];
    let yes = ["--host", "127.0.0.1", "--port", port.as_str(), "--yes"];
    let credential = br#"{"credential":"private"}"#;
    for command in [
        &["skill-market", "shiju-install", "7", "--input", "-"][..],
        &["skill-market", "shiju-uninstall", "7", "--input", "-"][..],
    ] {
        let error =
            run_command(&common, command, credential).await.expect_err("install requires --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let search = br#"{"credential":"private","query":{"query":"rust cli","cursor":"c/1","page_size":5,"tag_ids":2,"company_tag_ids":5}}"#;
    let system =
        br#"{"credential":"private","query":{"page":1,"page_size":5,"name":"agent","tag_id":2}}"#;
    let maintained = br#"{"credential":"private","query":{"page":1,"page_size":5,"keyword":"ops","tag_ids":[2,1],"company_tag_ids":[5],"statuses":["draft","published"]}}"#;
    let market = br#"{"keyword":"rust cli","category":"dev","namespace":"ns/one","sort":"downloads","page":2,"page_size":5}"#;

    let shiju_reads = [
        (
            &["skill-market", "shiju-search", "--input", "-"][..],
            search.as_slice(),
            "/v1/shiju/skills/search?page_size=5&q=rust+cli&cursor=c%2F1&tag_ids=2&company_tag_ids=5",
        ),
        (
            &["skill-market", "shiju-installed", "--input", "-"][..],
            credential.as_slice(),
            "/v1/shiju/skills/installed",
        ),
        (
            &["skill-market", "shiju-system", "--input", "-"][..],
            system.as_slice(),
            "/v1/shiju/skills/system?page=1&page_size=5&name=agent&tag_id=2",
        ),
        (
            &["skill-market", "shiju-maintained", "--input", "-"][..],
            maintained.as_slice(),
            "/v1/shiju/skills/maintained?page=1&page_size=5&keyword=ops&tag_ids=2%2C1&company_tag_ids=5&statuses=draft%2Cpublished",
        ),
        (
            &["skill-market", "shiju-detail", "7", "--input", "-"][..],
            credential.as_slice(),
            "/v1/shiju/skills/7",
        ),
        (
            &["skill-market", "shiju-versions", "7", "--input", "-"][..],
            credential.as_slice(),
            "/v1/shiju/skills/7/versions",
        ),
    ];
    for (command, input, path) in shiju_reads {
        let _ = run_command(&common, command, input).await;
        assert_request(&requests, "GET", path, Value::Null);
    }
    run_command(&yes, &["skill-market", "shiju-install", "7", "--input", "-"], credential)
        .await
        .unwrap();
    assert_request(&requests, "POST", "/v1/shiju/skills/7/install", Value::Null);
    run_command(&yes, &["skill-market", "shiju-uninstall", "7", "--input", "-"], credential)
        .await
        .unwrap();
    assert_request(&requests, "DELETE", "/v1/shiju/skills/7/install", Value::Null);

    let unified = [
        (&["skill-market", "sources"][..], b"".as_slice(), "/v1/skill-market/sources"),
        (
            &["skill-market", "recommended", "--source", "src"][..],
            b"".as_slice(),
            "/v1/skill-market/showcase/recommended?source=src",
        ),
        (
            &["skill-market", "skills", "--source", "src", "--input", "-"][..],
            market.as_slice(),
            "/v1/skill-market/skills?source=src&keyword=rust+cli&category=dev&namespace=ns%2Fone&sort=downloads&page=2&page_size=5",
        ),
        (
            &["skill-market", "categories", "--source", "src"][..],
            b"".as_slice(),
            "/v1/skill-market/categories?source=src",
        ),
        (
            &["skill-market", "detail", "--source", "src", "--namespace", "ns/one", "slug/one"][..],
            b"".as_slice(),
            "/v1/skill-market/skills/slug%2Fone?source=src&namespace=ns%2Fone",
        ),
        (
            &["skill-market", "file", "--source", "src", "--namespace", "ns/one", "slug/one"][..],
            b"".as_slice(),
            "/v1/skill-market/skills/slug%2Fone/file?source=src&namespace=ns%2Fone",
        ),
        (
            &["skill-market", "skillsets", "--source", "src", "--input", "-"][..],
            market.as_slice(),
            "/v1/skill-market/skillsets?source=src&keyword=rust+cli&category=dev&namespace=ns%2Fone&sort=downloads&page=2&page_size=5",
        ),
        (
            &["skill-market", "skillset-detail", "--source", "src", "set/one"][..],
            b"".as_slice(),
            "/v1/skill-market/skillsets/set%2Fone?source=src",
        ),
    ];
    for (command, input, path) in unified {
        let _ = run_command(&common, command, input).await;
        assert_request(&requests, "GET", path, Value::Null);
    }
}
