use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};

fn make_channel() -> IrcChannel {
    IrcChannel::new(IrcChannelConfig {
        server: "irc.example.com".to_string(),
        port: 6697,
        nickname: "bot".to_string(),
        username: None,
        channels: vec!["#vibewindow".to_string()],
        allowed_users: vec!["*".to_string()],
        server_password: None,
        nickserv_password: None,
        sasl_password: None,
        verify_tls: true,
    })
}

#[test]
fn parse_missing_space_after_prefix_returns_none() {
    assert!(IrcMessage::parse(":nick!user@host").is_none());
}

#[test]
fn parse_command_without_params() {
    let message = IrcMessage::parse("QUIT").unwrap();

    assert!(message.prefix.is_none());
    assert_eq!(message.command, "QUIT");
    assert!(message.params.is_empty());
}

#[test]
fn parse_multiple_middle_params_and_trailing_value() {
    let message =
        IrcMessage::parse(":server 005 bot CHANTYPES=#& PREFIX=(ov)@+ :supported").unwrap();

    assert_eq!(message.prefix.as_deref(), Some("server"));
    assert_eq!(message.command, "005");
    assert_eq!(message.params, vec!["bot", "CHANTYPES=#&", "PREFIX=(ov)@+", "supported"]);
}

#[test]
fn parse_preserves_leading_colon_in_trailing_value() {
    let message = IrcMessage::parse(":nick PRIVMSG #chan ::literal").unwrap();

    assert_eq!(message.params, vec!["#chan", ":literal"]);
}

#[test]
fn nick_returns_none_for_empty_prefix_nick() {
    let message = IrcMessage::parse(": PRIVMSG #chan :hello").unwrap();

    assert!(message.nick().is_none());
}

#[test]
fn sasl_plain_encoding_handles_padding_variants() {
    assert_eq!(encode_sasl_plain("", ""), "AAA=");
    assert_eq!(encode_sasl_plain("n", "p"), "AG4AcA==");
    assert_eq!(encode_sasl_plain("bot", "secret"), "AGJvdABzZWNyZXQ=");
}

#[test]
fn split_zero_max_bytes_joins_non_empty_lines() {
    let chunks = split_message("one\r\ntwo\n\nthree", 0);

    assert_eq!(chunks, vec!["one two three"]);
}

#[test]
fn split_zero_max_bytes_preserves_empty_result_for_blank_input() {
    let chunks = split_message("\r\n\n", 0);

    assert_eq!(chunks, vec![""]);
}

#[test]
fn split_advances_to_next_utf8_boundary_when_limit_is_inside_first_char() {
    let chunks = split_message("€a", 1);

    assert_eq!(chunks, vec!["€", "a"]);
}

#[test]
fn split_empty_crlf_lines_returns_single_empty_chunk() {
    let chunks = split_message("\r\n\r\n", 400);

    assert_eq!(chunks, vec![""]);
}

#[tokio::test]
async fn send_without_connection_reports_not_connected() {
    let channel = make_channel();
    let error =
        channel.send(&SendMessage::new("hello", "#vibewindow")).await.unwrap_err().to_string();

    assert_eq!(error, "IRC not connected");
}

#[test]
fn parse_whitespace_only_line_returns_none() {
    // Non-empty after CRLF trim, but yields no command after whitespace split.
    assert!(IrcMessage::parse("   ").is_none());
}

#[test]
fn sasl_plain_encoding_pads_single_trailing_byte() {
    // Input "\0a\0bc" is 5 bytes → final chunk has one byte → "==" padding.
    assert_eq!(encode_sasl_plain("a", "bc"), "AGEAYmM=");
}

#[test]
fn no_verify_accepts_any_server_certificate() {
    use rustls::client::danger::ServerCertVerifier;
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

    let cert = CertificateDer::from(vec![0u8, 1, 2, 3]);
    let server_name = ServerName::try_from("irc.example.com").unwrap();
    let now = UnixTime::since_unix_epoch(std::time::Duration::from_secs(1_700_000_000));

    let result = NoVerify.verify_server_cert(&cert, &[], &server_name, &[], now);

    assert!(result.is_ok());
}

#[test]
fn no_verify_advertises_supported_schemes() {
    use rustls::client::danger::ServerCertVerifier;

    let schemes = NoVerify.supported_verify_schemes();

    assert!(!schemes.is_empty());
}

/// Build a minimal valid `DigitallySignedStruct` (scheme + empty signature)
/// via the low-level codec, since its constructor is crate-private.
fn dummy_signed_struct() -> rustls::DigitallySignedStruct {
    use rustls::internal::msgs::codec::{Codec, Reader};

    // ECDSA_NISTP256_SHA256 = 0x0403, followed by a u16 length of 0.
    let encoded = [0x04u8, 0x03, 0x00, 0x00];
    let mut reader = Reader::init(&encoded);
    rustls::DigitallySignedStruct::read(&mut reader).unwrap()
}

#[test]
fn no_verify_accepts_tls12_and_tls13_signatures() {
    use rustls::client::danger::ServerCertVerifier;
    use rustls::pki_types::CertificateDer;

    let cert = CertificateDer::from(vec![9u8, 8, 7]);
    let dss = dummy_signed_struct();

    assert!(NoVerify.verify_tls12_signature(b"message", &cert, &dss).is_ok());
    assert!(NoVerify.verify_tls13_signature(b"message", &cert, &dss).is_ok());
}

#[tokio::test]
async fn connect_to_closed_port_fails() {
    // Bind then drop the listener so the port refuses connections.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let channel = IrcChannel::new(IrcChannelConfig {
        server: "127.0.0.1".to_string(),
        port,
        nickname: "bot".to_string(),
        username: None,
        channels: vec![],
        allowed_users: vec!["*".to_string()],
        server_password: None,
        nickserv_password: None,
        sasl_password: None,
        verify_tls: false,
    });

    assert!(channel.connect().await.is_err());
}

#[tokio::test]
async fn health_check_is_false_when_tls_handshake_fails() {
    // A plain TCP server that never speaks TLS makes the handshake fail,
    // so the connectivity probe must report unhealthy.
    // ClientConfig::builder() relies on a process-default crypto provider.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        // Accept one connection and immediately drop it.
        let _ = listener.accept().await;
    });

    let channel = IrcChannel::new(IrcChannelConfig {
        server: "127.0.0.1".to_string(),
        port,
        nickname: "bot".to_string(),
        username: None,
        channels: vec![],
        allowed_users: vec!["*".to_string()],
        server_password: None,
        nickserv_password: None,
        sasl_password: None,
        verify_tls: false,
    });

    assert!(!channel.health_check().await);
}

#[tokio::test]
async fn connect_with_certificate_verification_rejects_plain_server() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = listener.accept().await;
    });

    let channel = IrcChannel::new(IrcChannelConfig {
        server: "127.0.0.1".to_string(),
        port,
        nickname: "bot".to_string(),
        username: None,
        channels: vec![],
        allowed_users: vec!["*".to_string()],
        server_password: None,
        nickserv_password: None,
        sasl_password: None,
        verify_tls: true,
    });

    assert!(channel.connect().await.is_err());
}
