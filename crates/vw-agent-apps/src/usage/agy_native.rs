//! Read per-generation model names from Antigravity's native conversation database.
use prost::Message;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::{collections::HashMap, path::Path, time::Duration};

// Only the fields needed for the step -> generation -> model association are decoded.
#[derive(Clone, PartialEq, Message)]
struct StepMetadata {
    #[prost(message, optional, tag = "20")]
    generation: Option<GenerationReference>,
}

#[derive(Clone, PartialEq, Message)]
struct GenerationReference {
    // Protobuf omits the scalar when the generation index is zero.
    #[prost(uint64, tag = "3")]
    index: u64,
}

#[derive(Clone, PartialEq, Message)]
struct GenerationMetadata {
    #[prost(message, optional, tag = "1")]
    response: Option<ResponseMetadata>,
}

#[derive(Clone, PartialEq, Message)]
struct ResponseMetadata {
    #[prost(string, tag = "19")]
    model: String,
}

// Field 21 carries the tiered ID (`gemini-4-argon-high`) that matches `Model Selection`;
// older builds leave it out or store a display name. It is decoded on its own so an
// unexpected value there can never hide field 19.
#[derive(Clone, PartialEq, Message)]
struct GenerationModelId {
    #[prost(message, optional, tag = "1")]
    response: Option<ResponseModelId>,
}

#[derive(Clone, PartialEq, Message)]
struct ResponseModelId {
    #[prost(string, tag = "21")]
    model_id: String,
}

// Field 11 is the wait for the first token and field 12 the rest of the stream; both match the
// step's own created / first-output / done timestamps to the millisecond. Decoded on their own
// so an unexpected wire type there can never hide the model name.
#[derive(Clone, PartialEq, Message)]
struct GenerationTiming {
    #[prost(message, optional, tag = "1")]
    response: Option<ResponseTiming>,
}

#[derive(Clone, PartialEq, Message)]
struct ResponseTiming {
    #[prost(message, optional, tag = "11")]
    first_token: Option<DurationValue>,
    #[prost(message, optional, tag = "12")]
    streaming: Option<DurationValue>,
}

/// `google.protobuf.Duration`.
#[derive(Clone, PartialEq, Message)]
struct DurationValue {
    #[prost(int64, tag = "1")]
    seconds: i64,
    #[prost(int32, tag = "2")]
    nanos: i32,
}

impl DurationValue {
    fn millis(&self) -> Option<i64> {
        if self.seconds < 0 || !(0..1_000_000_000).contains(&self.nanos) {
            return None;
        }
        let ms = self.seconds.checked_mul(1000)?.checked_add(i64::from(self.nanos) / 1_000_000)?;
        (ms >= 0).then_some(ms)
    }
}

/// Exact timing recorded by agy for one generation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Timing {
    pub latency_ms: i64,
    pub first_token_ms: i64,
}

#[derive(Default)]
pub(super) struct Generation {
    pub model: Option<String>,
    pub timing: Option<Timing>,
}

fn timing_of(bytes: &[u8]) -> Option<Timing> {
    let response = GenerationTiming::decode(bytes).ok()?.response?;
    let first_token_ms = response.first_token?.millis()?;
    let latency_ms = first_token_ms.checked_add(response.streaming?.millis()?)?;
    (latency_ms > 0).then_some(Timing { latency_ms, first_token_ms })
}

pub(super) fn generations_for_transcript(
    transcript: &Path,
    steps: &[u64],
) -> Result<HashMap<u64, Generation>, String> {
    // brain/<id>/.system_generated/logs/transcript_full.jsonl and conversations/<id>.db
    // share the same CLI/IDE/private-profile root.
    let session = transcript
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("Missing transcript session directory")?;
    let root = session
        .parent()
        .filter(|brain| brain.file_name().is_some_and(|name| name == "brain"))
        .and_then(Path::parent)
        .ok_or("Missing Antigravity root")?;
    let id = session.file_name().ok_or("Missing session id")?;
    let path = root.join("conversations").join(id).with_extension("db");
    crate::sessions::providers::agy::checked_path(&path)?;
    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err("Invalid Antigravity database".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(_) => return Err("Cannot inspect Antigravity database".into()),
    }
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    if !path.starts_with(root) {
        return Err("Conversation database is outside the Antigravity root".into());
    }
    read_generations(&path, steps).map_err(|error| error.to_string())
}

fn read_generations(path: &Path, steps: &[u64]) -> rusqlite::Result<HashMap<u64, Generation>> {
    let mut conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    // A busy native writer must not hold up the usage refresh. Normal SQLite reads
    // include the WAL; immutable mode would miss live generations.
    conn.busy_timeout(Duration::ZERO)?;
    let tx = conn.transaction()?;
    let mut step_query = tx.prepare("SELECT metadata FROM steps WHERE idx=?1 AND step_type=15")?;
    let mut generation_query = tx.prepare("SELECT data FROM gen_metadata WHERE idx=?1")?;
    let mut generations = HashMap::new();
    for &step in steps {
        let Ok(index) = i64::try_from(step) else {
            continue;
        };
        let metadata: Option<Vec<u8>> =
            step_query.query_row([index], |row| row.get(0)).optional()?.flatten();
        let Some(reference) = metadata
            .and_then(|bytes| StepMetadata::decode(bytes.as_slice()).ok())
            .and_then(|metadata| metadata.generation)
        else {
            continue;
        };
        let Ok(generation) = i64::try_from(reference.index) else {
            continue;
        };
        let data: Option<Vec<u8>> =
            generation_query.query_row([generation], |row| row.get(0)).optional()?.flatten();
        let Some(bytes) = data else {
            continue;
        };
        let tiered = GenerationModelId::decode(bytes.as_slice())
            .ok()
            .and_then(|metadata| metadata.response)
            .map(|response| response.model_id);
        let base = GenerationMetadata::decode(bytes.as_slice())
            .ok()
            .and_then(|metadata| metadata.response)
            .map(|response| response.model);
        let model = [tiered, base]
            .into_iter()
            .flatten()
            .map(|model| model.trim().to_string())
            .find(|model| {
                !model.is_empty() && !model.contains('\0') && !model.eq_ignore_ascii_case("unknown")
            });
        let timing = timing_of(&bytes);
        if model.is_some() || timing.is_some() {
            generations.insert(step, Generation { model, timing });
        }
    }
    Ok(generations)
}
