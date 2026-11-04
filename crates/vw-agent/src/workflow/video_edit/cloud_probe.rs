//! Normalize the verified media_probe projection, rather than assuming raw ffprobe JSON.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
pub(crate) fn normalize_media_probe(value: Value) -> Result<Value> {
    let duration = value["durationSec"].as_f64().context("missing probed duration")?;
    ensure!(duration.is_finite() && duration > 0.0, "invalid probed duration");
    let mut streams = Vec::new();
    for stream in value["streams"].as_array().context("probe missing streams")? {
        // codec_type is the previously supported raw ffprobe projection; type is
        // the official service projection independently observed on live probes.
        let kind = stream["type"]
            .as_str()
            .or_else(|| stream["codec_type"].as_str())
            .context("probe stream missing type")?;
        let mut normalized = stream.clone();
        normalized["codec_type"] = json!(kind);
        if kind == "video" {
            for field in ["width", "height"] {
                let Some(v) = stream.get(field).or_else(|| value["video"].get(field)) else {
                    continue;
                };
                ensure!(v.as_u64().is_some_and(|n| n > 0), "invalid video dimensions");
                normalized[field] = v.clone();
            }
            if let Some(rate) = stream
                .get("avg_frame_rate")
                .or_else(|| stream.get("fps"))
                .or_else(|| value["video"].get("fps"))
                .or_else(|| value.get("fps"))
            {
                normalized["avg_frame_rate"] = rate.clone();
            }
        }
        streams.push(normalized);
    }
    Ok(json!({"durationSec":duration,"streams":streams}))
}
