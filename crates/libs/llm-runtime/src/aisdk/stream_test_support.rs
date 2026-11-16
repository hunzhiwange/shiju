//! Shared fake SDK provider; request execution is supplied by the parent test module.
use super::*;

pub(super) type ChunkResult = aisdk::Result<Vec<AiStreamChunk>>;
type BoxedProviderStream = Pin<Box<dyn Stream<Item = ChunkResult> + Send>>;

#[derive(Clone, Debug)]
pub(super) struct FakeModel {
    open_error: Option<AiError>,
    items: Vec<ChunkResult>,
    pending_stream: bool,
    captured_options: Arc<Mutex<Option<LanguageModelOptions>>>,
}

impl FakeModel {
    pub(super) fn with_items(items: Vec<ChunkResult>) -> (Self, Arc<Mutex<Option<LanguageModelOptions>>>) {
        let captured_options = Arc::new(Mutex::new(None));
        (
            Self {
                open_error: None,
                items,
                pending_stream: false,
                captured_options: captured_options.clone(),
            },
            captured_options,
        )
    }

    #[allow(dead_code)] // Used by the host failure suite.
    pub(super) fn with_open_error(error: AiError) -> Self {
        Self {
            open_error: Some(error),
            items: Vec::new(),
            pending_stream: false,
            captured_options: Arc::new(Mutex::new(None)),
        }
    }

    #[allow(dead_code)] // Used by the host cancellation suite.
    pub(super) fn with_pending_stream() -> Self {
        Self {
            open_error: None,
            items: Vec::new(),
            pending_stream: true,
            captured_options: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl LanguageModel for FakeModel {
    fn name(&self) -> String {
        "fake".to_string()
    }

    async fn generate_text(
        &mut self,
        _options: LanguageModelOptions,
    ) -> aisdk::Result<LanguageModelResponse> {
        Ok(LanguageModelResponse::new(""))
    }

    async fn stream_text(
        &mut self,
        options: LanguageModelOptions,
    ) -> aisdk::Result<BoxedProviderStream> {
        *self.captured_options.lock().expect("capture options") = Some(options);

        if let Some(error) = self.open_error.clone() {
            return Err(error);
        }

        if self.pending_stream {
            return Ok(Box::pin(futures_util::stream::pending()));
        }

        Ok(Box::pin(futures_util::stream::iter(self.items.clone())))
    }
}

impl ToolCallSupport for FakeModel {}

pub(super) fn tool_call(id: &str, name: &str, input: Value) -> ToolCallInfo {
    let mut info = ToolCallInfo::new(name);
    info.id(id);
    info.input(input);
    info
}

pub(super) fn done(content: AiContent, usage: Option<Usage>) -> AiStreamChunk {
    AiStreamChunk::Done(AssistantMessage::new(content, usage))
}

pub(super) async fn run_fake_stream(
    provider_id: &str,
    request_url: &str,
    model: FakeModel,
    merged_options: &Value,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
) -> (Result<(), Error>, Vec<StreamEvent>) {
    let mut events = Vec::new();
    let tools = HashMap::new();

    let result = do_stream_request_aisdk_with_model(
        provider_id.to_string(),
        model,
        request_url.to_string(),
        true,
        false,
        vec![Message::User("hello".into())],
        None,
        &tools,
        None,
        None,
        None,
        merged_options,
        0,
        abort,
        &mut |event| events.push(event),
    )
    .await;

    (result, events)
}
