//! Bounded JavaScript execution for the two host-side stages of a usage query.

use std::{fmt, time::Duration};

use rquickjs::{
    Context, Ctx, Exception, Function, Object, Runtime, String as JsString, Value as JsValue,
};
use serde_json::Value;

use super::config::PreparedUsageScript;

const CPU_BUDGET: Duration = Duration::from_secs(5);
pub(super) const MEMORY_LIMIT: usize = 16 * 1024 * 1024;
const STACK_LIMIT: usize = 256 * 1024;

/// Errors contain only a stable classification. Script text, exceptions, and
/// response payloads can contain credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    Unavailable,
    TimedOut,
    SourceTooLarge,
    Evaluation,
    InvalidRequest,
    InvalidResponse,
    InvalidResult,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "usage script runtime unavailable",
            Self::TimedOut => "usage script exceeded its CPU budget",
            Self::SourceTooLarge => "usage script source exceeds the memory budget",
            Self::Evaluation => "usage script evaluation failed",
            Self::InvalidRequest => "usage script request construction failed",
            Self::InvalidResponse => "usage script response is invalid or too large",
            Self::InvalidResult => "usage script result extraction failed",
        })
    }
}

impl std::error::Error for RuntimeError {}

impl PreparedUsageScript<'_> {
    /// Produce a request candidate for host-side authorization and validation.
    pub async fn build_request(&self) -> Result<Value, RuntimeError> {
        let source = bounded_source(self.source())?;
        tokio::task::spawn_blocking(move || {
            run_stage(&source, |ctx, config| {
                let request: Object =
                    config.get("request").map_err(|_| RuntimeError::InvalidRequest)?;
                to_json(ctx.json_stringify(request), RuntimeError::InvalidRequest)
            })
        })
        .await
        .map_err(|_| RuntimeError::Unavailable)?
    }

    /// Extract a JSON candidate from the response received by the host.
    pub(super) async fn extract(&self, response: &str) -> Result<Value, RuntimeError> {
        if response.len() >= MEMORY_LIMIT {
            return Err(RuntimeError::InvalidResponse);
        }
        let source = bounded_source(self.source())?;
        let response = response.to_owned();
        tokio::task::spawn_blocking(move || {
            run_stage(&source, |ctx, config| {
                let extractor: Function =
                    config.get("extractor").map_err(|_| RuntimeError::InvalidResult)?;
                let response =
                    ctx.json_parse(response).map_err(|_| RuntimeError::InvalidResponse)?;
                let result: JsValue =
                    extractor.call((response,)).map_err(|_| RuntimeError::InvalidResult)?;
                // JSON.stringify would turn non-finite numbers into null, hiding them
                // from validation of the optional quantities.
                let replacer = Function::new(ctx.clone(), reject_non_finite)
                    .map_err(|_| RuntimeError::InvalidResult)?;
                to_json(ctx.json_stringify_replacer(result, replacer), RuntimeError::InvalidResult)
            })
        })
        .await
        .map_err(|_| RuntimeError::Unavailable)?
    }
}

fn bounded_source(source: &str) -> Result<String, RuntimeError> {
    if source.len() >= MEMORY_LIMIT {
        return Err(RuntimeError::SourceTooLarge);
    }
    Ok(source.to_owned())
}

fn run_stage(
    source: &str,
    execute: impl for<'js> FnOnce(Ctx<'js>, Object<'js>) -> Result<Value, RuntimeError>,
) -> Result<Value, RuntimeError> {
    let started = thread_cpu_time()?;
    let runtime = Runtime::new().map_err(|_| RuntimeError::Unavailable)?;
    runtime.set_memory_limit(MEMORY_LIMIT);
    runtime.set_max_stack_size(STACK_LIMIT);
    runtime.set_interrupt_handler(Some(Box::new(move || check_cpu_budget(started).is_err())));

    let context = Context::full(&runtime).map_err(|_| RuntimeError::Unavailable)?;
    let result = context.with(|ctx| {
        let config = ctx.eval(source).map_err(|_| RuntimeError::Evaluation)?;
        execute(ctx, config)
    });
    // Check after native engine operations as well, since they may not poll the
    // interrupt handler before returning.
    check_cpu_budget(started)?;
    result
}

fn reject_non_finite<'js>(
    ctx: Ctx<'js>,
    _key: JsValue<'js>,
    value: JsValue<'js>,
) -> rquickjs::Result<JsValue<'js>> {
    if value.as_float().is_some_and(|number| !number.is_finite()) {
        return Err(Exception::throw_type(&ctx, "non-finite usage number"));
    }
    Ok(value)
}

fn to_json(
    json: rquickjs::Result<Option<JsString<'_>>>,
    error: RuntimeError,
) -> Result<Value, RuntimeError> {
    let json = json.map_err(|_| error)?.ok_or(error)?;
    let json = json.to_string().map_err(|_| error)?;
    serde_json::from_str(&json).map_err(|_| error)
}

fn check_cpu_budget(started: Duration) -> Result<(), RuntimeError> {
    if thread_cpu_time()?.saturating_sub(started) >= CPU_BUDGET {
        return Err(RuntimeError::TimedOut);
    }
    Ok(())
}

fn thread_cpu_time() -> Result<Duration, RuntimeError> {
    let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: time is a valid writable timespec, and the clock tracks only the
    // current blocking worker, excluding queue time and other gateway work.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) } != 0 {
        return Err(RuntimeError::Unavailable);
    }
    Ok(Duration::new(time.tv_sec as u64, time.tv_nsec as u32))
}
