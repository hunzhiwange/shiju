use axum::{body::Body, extract::ConnectInfo, http::Request};
use tower::ServiceExt;

use super::*;

#[test]
fn ssh_aliases_follow_includes_without_importing_patterns() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".ssh");
    fs::create_dir_all(config.join("hosts")).unwrap();
    fs::write(
        config.join("config"),
        "Host first * !ignored\nInclude hosts/*.conf\nHost=last # comment\n",
    )
    .unwrap();
    fs::write(
        config.join("hosts/team.conf"),
        "Host \"team-box\" invalid;command\nInclude config\n",
    )
    .unwrap();

    assert_eq!(
        discover_aliases(home.path()).into_iter().collect::<Vec<_>>(),
        ["first", "last", "team-box"],
    );
}

#[test]
fn ssh_destination_cannot_be_interpreted_as_an_option_or_shell_fragment() {
    for destination in ["-F", "person@-F", "person@host;cmd", "person@@host", "person@host -v"] {
        assert!(!valid_destination(destination));
    }
    for destination in ["work.example.org", "person@host", "person@[2001:db8::1]"] {
        assert!(valid_destination(destination));
    }
}

#[tokio::test]
async fn forwarded_requests_cannot_list_gateway_ssh_connections() {
    let response = super::super::router::<()>()
        .layer(Extension(TenantScope::default()))
        .layer(Extension(LocalWorkbenchGateway::default()))
        .oneshot(
            Request::builder()
                .uri("/cli-workbench/ssh")
                .extension(ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 1234))))
                .header("x-forwarded-for", "192.0.2.10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
