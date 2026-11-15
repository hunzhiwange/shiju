use std::{collections::HashMap, io::{Read, Write}, net::TcpListener, sync::mpsc::{self, Receiver}, thread::JoinHandle, time::Duration};
use vw_shared::provider::types::{ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost, ModelCostCache, ModelLimit, ProviderSource};

// 单次请求服务器既返回受控响应，也把原始请求交给测试线程核对认证头和路径。
pub(crate) struct TestServer {
    pub(crate) api_base: String,
    pub(crate) request: Receiver<String>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("test server should stop");
        }
    }
}

pub(crate) fn serve_once(status: &str, headers: &[(&str, &str)], body: &str, delay: Duration) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener.local_addr().expect("test listener should have an address");
    let (request_tx, request_rx) = mpsc::channel();
    let response_headers =
        headers.iter().map(|(name, value)| format!("{name}: {value}\r\n")).collect::<String>();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{response_headers}Connection: close\r\n\r\n{body}",
        body.len()
    );

    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept one request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout should be configured");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let read = stream.read(&mut chunk).expect("request should be readable");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        request_tx
            .send(String::from_utf8(request).expect("request should be UTF-8"))
            .expect("request should be observed");
        std::thread::sleep(delay);
        let _ = stream.write_all(response.as_bytes());
    });

    TestServer { api_base: format!("http://{address}"), request: request_rx, thread: Some(thread) }
}

pub(crate) fn test_model(id: &str, status: &str) -> Model {
    Model {
        id: id.to_string(),
        provider_id: "test-provider".to_string(),
        api: ApiInfo {
            protocols: None,
            id: id.to_string(),
            url: "https://static.example/v1".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: format!("Static {id}"),
        family: Some("static-family".to_string()),
        capabilities: Capabilities {
            native_tool_search: false,
            catalog_execution: None,
            temperature: true,
            reasoning: true,
            reasoning_options: None,
            attachment: true,
            toolcall: true,
            input: CapabilityIO { text: true, audio: false, image: true, video: false, pdf: false },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Bool(false),
        },
        cost: ModelCost {
            input: 1.0,
            output: 2.0,
            cache: ModelCostCache { read: 0.1, write: 0.2 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 32_000, input: Some(16_000), output: 8_000 },
        status: status.to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    }
}

pub(crate) fn test_provider() -> Info {
    Info {
        id: "test-provider".to_string(),
        name: "Test Provider".to_string(),
        source: ProviderSource::Config,
        env: vec![],
        key: Some("secret-token".to_string()),
        options: HashMap::new(),
        models: HashMap::from([
            ("explicit-disabled".to_string(), test_model("explicit-disabled", "disabled")),
            ("static-model".to_string(), test_model("static-model", "disabled")),
        ]),
    }
}
