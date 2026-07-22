use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn one_shot_image_server(
        status_line: &'static str,
        content_type: Option<&'static str>,
        content_length: Option<usize>,
        body: Vec<u8>,
    ) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind image server");
        let addr = listener.local_addr().expect("read image server address");

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept image request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await;

            let mut response = format!(
                "{status_line}\r\ncontent-length: {}\r\nconnection: close\r\n",
                content_length.unwrap_or(body.len())
            );
            if let Some(content_type) = content_type {
                response.push_str("content-type: ");
                response.push_str(content_type);
                response.push_str("\r\n");
            }
            response.push_str("\r\n");

            stream.write_all(response.as_bytes()).await.expect("write image response headers");
            stream.write_all(&body).await.expect("write image response body");
        });

        format!("http://{addr}/image")
    }

    #[test]
    fn parse_image_markers_extracts_multiple_markers() {
        let input = "Check this [IMAGE:/tmp/a.png] and this [IMAGE:https://example.com/b.jpg]";
        let (cleaned, refs) = parse_image_markers(input);

        assert_eq!(cleaned, "Check this  and this");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], "/tmp/a.png");
        assert_eq!(refs[1], "https://example.com/b.jpg");
    }

    #[test]
    fn parse_image_markers_keeps_invalid_empty_marker() {
        let input = "hello [IMAGE:] world";
        let (cleaned, refs) = parse_image_markers(input);

        assert_eq!(cleaned, "hello [IMAGE:] world");
        assert!(refs.is_empty());
    }

    #[test]
    fn parse_image_markers_keeps_unclosed_marker_as_text() {
        let input = "hello [IMAGE:/tmp/open.png";
        let (cleaned, refs) = parse_image_markers(input);

        assert_eq!(cleaned, input);
        assert!(refs.is_empty());
    }

    #[test]
    fn parse_image_markers_trims_text_and_reference_whitespace() {
        let (cleaned, refs) = parse_image_markers("  before [IMAGE:  image.png  ] after  ");

        assert_eq!(cleaned, "before  after");
        assert_eq!(refs, vec!["image.png"]);
    }

    #[test]
    fn count_image_markers_only_counts_user_messages() {
        let messages = vec![
            ChatMessage::system("[IMAGE:system.png]"),
            ChatMessage::user("[IMAGE:first.png] [IMAGE:second.png]"),
            ChatMessage::assistant("[IMAGE:assistant.png]"),
            ChatMessage::user("plain text"),
        ];

        assert_eq!(count_image_markers(&messages), 2);
        assert!(contains_image_markers(&messages));
    }

    #[test]
    fn contains_image_markers_rejects_non_user_markers_and_empty_markers() {
        let messages =
            vec![ChatMessage::assistant("[IMAGE:assistant.png]"), ChatMessage::user("[IMAGE:   ]")];

        assert!(!contains_image_markers(&messages));
    }

    #[tokio::test]
    async fn prepare_messages_normalizes_local_image_to_data_uri() {
        let temp = tempfile::tempdir().unwrap();
        let image_path = temp.path().join("sample.png");

        // Minimal PNG signature bytes are enough for MIME detection.
        std::fs::write(&image_path, [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']).unwrap();

        let messages = vec![ChatMessage::user(format!(
            "Please inspect this screenshot [IMAGE:{}]",
            image_path.display()
        ))];

        let prepared =
            prepare_messages_for_provider(&messages, &MultimodalConfig::default()).await.unwrap();

        assert!(prepared.contains_images);
        assert_eq!(prepared.messages.len(), 1);

        let (cleaned, refs) = parse_image_markers(&prepared.messages[0].content);
        assert_eq!(cleaned, "Please inspect this screenshot");
        assert_eq!(refs.len(), 1);
        assert!(refs[0].starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn prepare_messages_rejects_too_many_images() {
        let messages =
            vec![ChatMessage::user("[IMAGE:/tmp/1.png]\n[IMAGE:/tmp/2.png]".to_string())];

        let config =
            MultimodalConfig { max_images: 1, max_image_size_mb: 5, allow_remote_fetch: false };

        let error = prepare_messages_for_provider(&messages, &config)
            .await
            .expect_err("should reject image count overflow");

        assert!(error.to_string().contains("multimodal image limit exceeded"));
    }

    #[tokio::test]
    async fn prepare_messages_rejects_remote_url_when_disabled() {
        let messages =
            vec![ChatMessage::user("Look [IMAGE:https://example.com/img.png]".to_string())];

        let error = prepare_messages_for_provider(&messages, &MultimodalConfig::default())
            .await
            .expect_err("should reject remote image URL when fetch is disabled");

        assert!(error.to_string().contains("multimodal remote image fetch is disabled"));
    }

    #[tokio::test]
    async fn normalize_image_reference_reports_malformed_remote_url() {
        let config =
            MultimodalConfig { max_images: 4, max_image_size_mb: 5, allow_remote_fetch: true };
        let client = Client::new();

        let error = normalize_image_reference("http://[::1", &config, 1024, &client)
            .await
            .expect_err("malformed URL should fail before network access");

        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::RemoteFetchFailed { input, reason })
                if input == "http://[::1" && !reason.is_empty()
        ));
    }

    #[tokio::test]
    async fn normalize_remote_image_accepts_http_image_and_canonicalizes_header() {
        let bytes = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        let source = one_shot_image_server(
            "HTTP/1.1 200 OK",
            Some(" IMAGE/PNG ; charset=binary "),
            None,
            bytes.clone(),
        )
        .await;

        let normalized = normalize_remote_image(&source, bytes.len(), &Client::new())
            .await
            .expect("remote PNG should normalize");

        assert_eq!(normalized, format!("data:image/png;base64,{}", STANDARD.encode(bytes)));
    }

    #[tokio::test]
    async fn normalize_remote_image_rejects_http_status_and_content_length_limit() {
        let not_found =
            one_shot_image_server("HTTP/1.1 404 Not Found", Some("image/png"), None, Vec::new())
                .await;
        let status_error = normalize_remote_image(&not_found, 1024, &Client::new())
            .await
            .expect_err("non-success remote response should fail");

        assert!(matches!(
            status_error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::RemoteFetchFailed { input, reason })
                if input == &not_found && reason == "HTTP 404 Not Found"
        ));

        let oversized =
            one_shot_image_server("HTTP/1.1 200 OK", Some("image/png"), Some(3), Vec::new()).await;
        let size_error = normalize_remote_image(&oversized, 2, &Client::new())
            .await
            .expect_err("oversized content-length should fail before reading body");

        assert!(matches!(
            size_error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::ImageTooLarge {
                input,
                size_bytes: 3,
                max_bytes: 2,
            }) if input == &oversized
        ));
    }

    #[tokio::test]
    async fn normalize_remote_image_rejects_unknown_mime_after_download() {
        let source =
            one_shot_image_server("HTTP/1.1 200 OK", None, None, b"not an image".to_vec()).await;

        let error = normalize_remote_image(&source, 1024, &Client::new())
            .await
            .expect_err("remote bytes without detectable MIME should fail");

        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::UnsupportedMime { input, mime })
                if input == &source && mime == "unknown"
        ));
    }

    #[tokio::test]
    async fn prepare_messages_rejects_oversized_local_image() {
        let temp = tempfile::tempdir().unwrap();
        let image_path = temp.path().join("big.png");

        let bytes = vec![0u8; 1024 * 1024 + 1];
        std::fs::write(&image_path, bytes).unwrap();

        let messages = vec![ChatMessage::user(format!("[IMAGE:{}]", image_path.display()))];
        let config =
            MultimodalConfig { max_images: 4, max_image_size_mb: 1, allow_remote_fetch: false };

        let error = prepare_messages_for_provider(&messages, &config)
            .await
            .expect_err("should reject oversized local image");

        assert!(error.to_string().contains("multimodal image size limit exceeded"));
    }

    #[tokio::test]
    async fn prepare_messages_preserves_messages_when_no_user_images_exist() {
        let messages = vec![
            ChatMessage::system("system prompt"),
            ChatMessage::assistant("assistant [IMAGE:not-user.png]"),
            ChatMessage::user("plain user text"),
        ];

        let prepared =
            prepare_messages_for_provider(&messages, &MultimodalConfig::default()).await.unwrap();

        assert!(!prepared.contains_images);
        assert_eq!(prepared.messages.len(), messages.len());
        for (actual, expected) in prepared.messages.iter().zip(&messages) {
            assert_eq!(actual.role, expected.role);
            assert_eq!(actual.content, expected.content);
        }
    }

    #[tokio::test]
    async fn prepare_messages_normalizes_data_uri_and_preserves_message_order() {
        let png_bytes = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        let payload = STANDARD.encode(png_bytes);
        let messages = vec![
            ChatMessage::system("system prompt"),
            ChatMessage::user("plain user text"),
            ChatMessage::user(format!("  inspect [IMAGE:data:IMAGE/PNG;base64,{payload}] now  ")),
            ChatMessage::assistant("assistant response"),
        ];

        let prepared =
            prepare_messages_for_provider(&messages, &MultimodalConfig::default()).await.unwrap();

        assert!(prepared.contains_images);
        assert_eq!(prepared.messages.len(), 4);
        assert_eq!(prepared.messages[0].role, "system");
        assert_eq!(prepared.messages[0].content, "system prompt");
        assert_eq!(prepared.messages[1].role, "user");
        assert_eq!(prepared.messages[1].content, "plain user text");
        assert_eq!(prepared.messages[2].role, "user");
        assert_eq!(
            prepared.messages[2].content,
            format!("inspect  now\n\n[IMAGE:data:image/png;base64,{payload}]")
        );
        assert_eq!(prepared.messages[3].role, "assistant");
        assert_eq!(prepared.messages[3].content, "assistant response");
    }

    #[tokio::test]
    async fn prepare_messages_rejects_invalid_data_uri_without_external_io() {
        let messages = vec![ChatMessage::user("[IMAGE:data:image/png,not-base64]")];

        let error = prepare_messages_for_provider(&messages, &MultimodalConfig::default())
            .await
            .expect_err("non-base64 data URI should fail");

        assert!(error.to_string().contains("only base64 data URIs are supported"));
    }

    #[tokio::test]
    async fn normalize_image_reference_rejects_missing_local_source() {
        let temp = tempfile::tempdir().unwrap();
        let missing_path = temp.path().join("missing.png");
        let source = missing_path.to_str().unwrap();
        let client = Client::new();

        let error = normalize_image_reference(source, &MultimodalConfig::default(), 1024, &client)
            .await
            .expect_err("missing local source should fail");

        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::ImageSourceNotFound { input }) if input.as_str() == source
        ));
    }

    #[tokio::test]
    async fn normalize_local_image_detects_magic_when_extension_is_unknown() {
        let temp = tempfile::tempdir().unwrap();
        let image_path = temp.path().join("sample.bin");
        let bytes = [0xff, 0xd8, 0xff, 0x00];
        std::fs::write(&image_path, bytes).unwrap();

        let normalized =
            normalize_local_image(image_path.to_str().unwrap(), bytes.len()).await.unwrap();

        assert_eq!(normalized, format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)));
    }

    #[tokio::test]
    async fn normalize_local_image_rejects_unknown_file_type() {
        let temp = tempfile::tempdir().unwrap();
        let image_path = temp.path().join("sample.bin");
        std::fs::write(&image_path, b"not an image").unwrap();

        let error = normalize_local_image(image_path.to_str().unwrap(), 1024)
            .await
            .expect_err("unknown file type should fail");

        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::UnsupportedMime { input, mime })
                if input.as_str() == image_path.to_str().unwrap() && mime.as_str() == "unknown"
        ));
    }

    #[test]
    fn extract_ollama_image_payload_supports_data_uris() {
        let payload = extract_ollama_image_payload("data:image/png;base64,abcd==")
            .expect("payload should be extracted");
        assert_eq!(payload, "abcd==");
    }

    #[test]
    fn extract_ollama_image_payload_handles_invalid_and_plain_references() {
        assert_eq!(extract_ollama_image_payload("  raw-payload  "), Some("raw-payload".into()));
        assert_eq!(extract_ollama_image_payload("data:image/png;base64,  "), None);
        assert_eq!(extract_ollama_image_payload("data:image/png;base64"), None);
        assert_eq!(extract_ollama_image_payload("   "), None);
    }

    #[test]
    fn compose_multimodal_message_formats_text_and_multiple_images() {
        let refs = vec!["data:image/png;base64,one".into(), "data:image/jpeg;base64,two".into()];

        assert_eq!(
            compose_multimodal_message("  describe these  ", &refs),
            "describe these\n\n[IMAGE:data:image/png;base64,one]\n[IMAGE:data:image/jpeg;base64,two]"
        );
        assert_eq!(
            compose_multimodal_message("   ", &refs[..1]),
            "[IMAGE:data:image/png;base64,one]"
        );
    }

    #[test]
    fn normalize_data_uri_canonicalizes_mime_and_payload() {
        let bytes = [0x89, b'P', b'N', b'G'];
        let payload = STANDARD.encode(bytes);

        let normalized =
            normalize_data_uri(&format!("data:IMAGE/PNG;base64,  {payload}  "), bytes.len())
                .unwrap();

        assert_eq!(normalized, format!("data:image/png;base64,{payload}"));
    }

    #[test]
    fn normalize_data_uri_rejects_malformed_inputs() {
        let cases = [
            ("data:image/png;base64", "expected data URI payload"),
            ("data:image/png,abcd", "only base64 data URIs are supported"),
            ("data:image/svg+xml;base64,PHN2Zz4=", "MIME type is not allowed"),
            ("data:image/png;base64,%%%", "invalid base64 payload"),
        ];

        for (source, expected_message) in cases {
            let error =
                normalize_data_uri(source, 1024).expect_err("malformed data URI should fail");
            assert!(
                error.to_string().contains(expected_message),
                "unexpected error for {source}: {error}"
            );
        }
    }

    #[test]
    fn normalize_data_uri_rejects_decoded_payload_over_limit() {
        let source = format!("data:image/png;base64,{}", STANDARD.encode([1_u8, 2, 3]));

        let error =
            normalize_data_uri(&source, 2).expect_err("decoded payload should exceed limit");

        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::ImageTooLarge {
                input,
                size_bytes: 3,
                max_bytes: 2,
            }) if input.as_str() == source.as_str()
        ));
    }

    #[test]
    fn validate_size_accepts_boundary_and_reports_overflow() {
        assert!(validate_size("image", 10, 10).is_ok());

        let error = validate_size("image", 11, 10).expect_err("size above limit should fail");
        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::ImageTooLarge {
                input,
                size_bytes: 11,
                max_bytes: 10,
            }) if input.as_str() == "image"
        ));
    }

    #[test]
    fn validate_mime_allows_image_whitelist_and_rejects_active_content() {
        for mime in ALLOWED_IMAGE_MIME_TYPES {
            assert!(validate_mime("image", mime).is_ok(), "expected {mime} to be allowed");
        }

        let error = validate_mime("image.svg", "image/svg+xml")
            .expect_err("SVG should be rejected by the image whitelist");
        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::UnsupportedMime { input, mime })
                if input.as_str() == "image.svg" && mime.as_str() == "image/svg+xml"
        ));
    }

    #[test]
    fn normalize_content_type_removes_parameters_and_rejects_empty_value() {
        assert_eq!(
            normalize_content_type(" IMAGE/PNG ; charset=binary "),
            Some("image/png".into())
        );
        assert_eq!(normalize_content_type(" ; charset=binary"), None);
        assert_eq!(normalize_content_type(""), None);
    }

    #[test]
    fn mime_from_extension_is_case_insensitive_for_supported_types() {
        let cases = [
            ("PNG", Some("image/png")),
            ("jpg", Some("image/jpeg")),
            ("JPEG", Some("image/jpeg")),
            ("webp", Some("image/webp")),
            ("GIF", Some("image/gif")),
            ("bmp", Some("image/bmp")),
            ("svg", None),
        ];

        for (extension, expected) in cases {
            assert_eq!(mime_from_extension(extension), expected);
        }
    }

    #[test]
    fn mime_from_magic_identifies_supported_signatures_and_short_inputs() {
        let cases: &[(&[u8], Option<&str>)] = &[
            (&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'], Some("image/png")),
            (&[0xff, 0xd8, 0xff], Some("image/jpeg")),
            (b"GIF87a", Some("image/gif")),
            (b"GIF89a", Some("image/gif")),
            (b"RIFF1234WEBP", Some("image/webp")),
            (b"BM", Some("image/bmp")),
            (b"RIFFWEBP", None),
            (b"", None),
        ];

        for (bytes, expected) in cases {
            assert_eq!(mime_from_magic(bytes), *expected);
        }
    }

    #[test]
    fn detect_mime_prefers_header_then_extension_then_magic() {
        let png_magic = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

        assert_eq!(
            detect_mime(Some(Path::new("image.jpg")), &png_magic, Some("IMAGE/GIF; version=89a")),
            Some("image/gif".into())
        );
        assert_eq!(
            detect_mime(Some(Path::new("image.jpg")), &png_magic, None),
            Some("image/jpeg".into())
        );
        assert_eq!(
            detect_mime(Some(Path::new("image.unknown")), &png_magic, None),
            Some("image/png".into())
        );
        assert_eq!(
            detect_mime(Some(Path::new("image")), &png_magic, None),
            Some("image/png".into())
        );
        assert_eq!(detect_mime(None, b"unknown", None), None);
    }

    #[test]
    fn multimodal_io_errors_preserve_input_and_reason() {
        let remote = MultimodalError::RemoteFetchFailed {
            input: "https://example.invalid/image.png".into(),
            reason: "connection refused".into(),
        };
        let local = MultimodalError::LocalReadFailed {
            input: "/missing/image.png".into(),
            reason: "permission denied".into(),
        };

        assert_eq!(
            remote.to_string(),
            "failed to download remote image 'https://example.invalid/image.png': connection refused"
        );
        assert_eq!(
            local.to_string(),
            "failed to read local image '/missing/image.png': permission denied"
        );
    }
}
