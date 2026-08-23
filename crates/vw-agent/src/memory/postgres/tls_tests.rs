// 验证 PostgreSQL 可选 TLS 校验器的证书与签名处理边界。
use super::*;
use rustls::client::danger::ServerCertVerifier;
use rustls::internal::msgs::codec::{Codec, Reader};
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use std::time::Duration;

fn empty_ecdsa_signature() -> rustls::DigitallySignedStruct {
    // ECDSA_NISTP256_SHA256 (0x0403), followed by an empty signature vector.
    let encoded = [0x04, 0x03, 0x00, 0x00];
    let mut reader = Reader::init(&encoded);
    rustls::DigitallySignedStruct::read(&mut reader).expect("valid signed-struct encoding")
}

#[test]
fn no_cert_verifier_advertises_signature_schemes() {
    let schemes = NoCertVerifier.supported_verify_schemes();

    assert_eq!(
        schemes,
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    );
}

#[test]
fn no_cert_verifier_accepts_any_server_certificate() {
    let cert = CertificateDer::from(vec![0, 1, 2, 3]);
    let server_name = ServerName::try_from("postgres.example.test").unwrap();
    let now = UnixTime::since_unix_epoch(Duration::from_secs(1_700_000_000));

    let result = NoCertVerifier.verify_server_cert(&cert, &[], &server_name, &[], now);

    assert!(result.is_ok());
}

#[test]
fn no_cert_verifier_rejects_invalid_tls12_signature_input() {
    let cert = CertificateDer::from(vec![0, 1, 2, 3]);
    let signature = empty_ecdsa_signature();

    let result = NoCertVerifier.verify_tls12_signature(b"message", &cert, &signature);

    assert!(result.is_err());
}

#[test]
fn no_cert_verifier_rejects_invalid_tls13_signature_input() {
    let cert = CertificateDer::from(vec![0, 1, 2, 3]);
    let signature = empty_ecdsa_signature();

    let result = NoCertVerifier.verify_tls13_signature(b"message", &cert, &signature);

    assert!(result.is_err());
}
