use crate::leak_detector::{LeakDetector, LeakResult};

// 测试正常内容应该通过泄露检测
#[test]
fn clean_content_passes() {
    let detector = LeakDetector::new();
    let result = detector.scan("This is just some normal text");
    assert!(matches!(result, LeakResult::Clean));
}

// 测试能够检测到 Stripe API 密钥
#[test]
fn detects_stripe_keys() {
    let detector = LeakDetector::new();
    let content = concat!("My Stripe key is sk_test_", "1234567890abcdefghijklmnop");
    let result = detector.scan(content);
    match result {
        LeakResult::Detected { patterns, redacted } => {
            assert!(patterns.iter().any(|p| p.contains("Stripe")));
            assert!(redacted.contains("[REDACTED"));
        }
        LeakResult::Clean => panic!("Should detect Stripe key"),
    }
}

// 测试能够检测到 AWS 凭证
#[test]
fn detects_aws_credentials() {
    let detector = LeakDetector::new();
    let content = "AWS key: AKIAIOSFODNN7EXAMPLE";
    let result = detector.scan(content);
    match result {
        LeakResult::Detected { patterns, .. } => {
            assert!(patterns.iter().any(|p| p.contains("AWS")));
        }
        LeakResult::Clean => panic!("Should detect AWS key"),
    }
}

// 测试能够检测到 RSA 私钥
#[test]
fn detects_private_keys() {
    let detector = LeakDetector::new();
    let content = r#"
-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEA0ZPr5JeyVDonXsKhfq...
-----END RSA PRIVATE KEY-----
"#;
    let result = detector.scan(content);
    match result {
        LeakResult::Detected { patterns, redacted } => {
            assert!(patterns.iter().any(|p| p.contains("private key")));
            assert!(redacted.contains("[REDACTED_PRIVATE_KEY]"));
        }
        LeakResult::Clean => panic!("Should detect private key"),
    }
}

// 测试能够检测到 JWT 令牌
#[test]
fn detects_jwt_tokens() {
    let detector = LeakDetector::new();
    let content = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let result = detector.scan(content);
    match result {
        LeakResult::Detected { patterns, redacted } => {
            assert!(patterns.iter().any(|p| p.contains("JWT")));
            assert!(redacted.contains("[REDACTED_JWT]"));
        }
        LeakResult::Clean => panic!("Should detect JWT"),
    }
}

// 测试能够检测到数据库连接 URL
#[test]
fn detects_database_urls() {
    let detector = LeakDetector::new();
    let content = "DATABASE_URL=postgres://user:secretpassword@localhost:5432/mydb";
    let result = detector.scan(content);
    match result {
        LeakResult::Detected { patterns, .. } => {
            assert!(patterns.iter().any(|p| p.contains("PostgreSQL")));
        }
        LeakResult::Clean => panic!("Should detect database URL"),
    }
}

// 测试低敏感度设置能够跳过通用模式的检测
#[test]
fn low_sensitivity_skips_generic() {
    let detector = LeakDetector::with_sensitivity(0.3);
    let content = "secret=mygenericvalue123456";
    let result = detector.scan(content);
    assert!(matches!(result, LeakResult::Clean));
}
