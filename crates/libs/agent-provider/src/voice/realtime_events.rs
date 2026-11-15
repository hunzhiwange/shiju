//! Bounded service-event parsing and response/item/generation association.
use super::*;
impl RealtimeSession {
    pub(super) async fn next_inner(&mut self) -> Result<Vec<VoiceFrame>> {
        let value = self.receive().await?;
        let mut events = Vec::new();
        let kind = value["type"].as_str().ok_or(VoiceErrorCode::ProtocolError)?;
        let response = if kind == "response.done" {
            value["response"]["id"].as_str()
        } else {
            value["response_id"].as_str()
        };
        if response.is_some_and(|id| self.retired.contains(id)) {
            return Ok(vec![]);
        }
        if kind == "response.output_item.added" && self.response.as_deref() != response {
            return Err(VoiceErrorCode::ProtocolError);
        }
        if kind == "response.output_item.added" {
            let item_id = id(&value["item"], "id")?;
            if (value["item"]["type"] == "function_call" && self.items.contains_key(&item_id))
                || (value["item"]["type"] == "message" && self.handoffs.contains(&item_id))
            {
                return Err(VoiceErrorCode::ProtocolError);
            }
        }
        if self.handoffs.event(&value, self.generation)? {
            return Ok(vec![]);
        }
        match kind {
            "input_audio_buffer.speech_started" if self.mode == TurnMode::ServerVad => {
                let response_id = self.playback_response.clone().map(voice_id).transpose()?;
                let positions: Vec<_> = self
                    .items
                    .iter()
                    .filter(|(_, item)| Some(&item.response) == self.playback_response.as_ref())
                    .flat_map(|(id, item)| {
                        item.audio.iter().map(move |(index, (_, ms))| (id.clone(), *index, *ms))
                    })
                    .collect();
                for (item, index, ms) in positions {
                    self.truncate(item.as_ref(), index, ms).await?;
                }
                self.retire_response()?;
                events.push(VoiceEvent::GenerationChanged {
                    reason: GenerationReason::Vad,
                    response_id,
                    item_id: None,
                    content_index: None,
                });
            }
            "conversation.item.truncated" => {
                let item = id(&value, "item_id")?;
                let index = value["content_index"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or(VoiceErrorCode::ProtocolError)?;
                let ms = value["audio_end_ms"].as_u64().ok_or(VoiceErrorCode::ProtocolError)?;
                if self.pending_truncation.remove(&(item, index)) != Some(ms) {
                    return Err(VoiceErrorCode::ProtocolError);
                }
            }
            "input_audio_buffer.cleared" => {
                self.pending_clear =
                    self.pending_clear.checked_sub(1).ok_or(VoiceErrorCode::ProtocolError)?;
            }
            "session.created" | "session.updated" => return Err(VoiceErrorCode::ProtocolError),
            "response.created" => {
                let response = id(&value["response"], "id")?;
                if self.response.is_some()
                    || !self.seen_responses.insert(response.clone())
                    || self.seen_responses.len() > 256
                {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                self.playback_response = Some(response.clone());
                self.response = Some(response.clone());
                events.push(VoiceEvent::ResponseStarted { response_id: voice_id(response)? });
            }
            "response.output_item.added" => {
                let response = id(&value, "response_id")?;
                if self.retired.contains(&response) {
                    return Ok(vec![]);
                }
                if self.response.as_deref() != Some(&response) {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                let item = id(&value["item"], "id")?;
                if self.items.len() >= 256 {
                    return Err(VoiceErrorCode::Backpressure);
                }
                if self.items.contains_key(&item) || value["item"]["type"] != "message" {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                self.items.insert(
                    item,
                    Item { generation: self.generation, response, audio: HashMap::new() },
                );
            }
            "input_audio_buffer.committed" => {
                let item = id(&value, "item_id")?;
                if self.items.len() >= 256 {
                    return Err(VoiceErrorCode::Backpressure);
                }
                if self.items.contains_key(&item) {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                self.items.insert(
                    item,
                    Item {
                        generation: self.generation,
                        response: String::new(),
                        audio: HashMap::new(),
                    },
                );
                self.input_samples = 0;
            }
            "response.output_audio.delta"
            | "response.output_audio.done"
            | "response.output_audio_transcript.delta"
            | "response.output_audio_transcript.done"
            | "conversation.item.input_audio_transcription.delta"
            | "conversation.item.input_audio_transcription.completed" => {
                let user = kind.starts_with("conversation.");
                let item = id(&value, "item_id")?;
                let index = value["content_index"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .filter(|v| *v < 16)
                    .ok_or(VoiceErrorCode::ProtocolError)?;
                if !user && self.retired.contains(&id(&value, "response_id")?) {
                    return Ok(vec![]);
                }
                let state = self.items.get_mut(&item).ok_or(VoiceErrorCode::ProtocolError)?;
                if user && (state.generation != self.generation || !state.response.is_empty()) {
                    return Ok(vec![]);
                }
                if !user {
                    let response = id(&value, "response_id")?;
                    if self.retired.contains(&response) {
                        return Ok(vec![]);
                    }
                    if state.response != response || self.response.as_deref() != Some(&response) {
                        return Err(VoiceErrorCode::ProtocolError);
                    }
                }
                if kind == "response.output_audio.delta" {
                    let encoded = value["delta"]
                        .as_str()
                        .filter(|s| s.len() <= 192 * 1024)
                        .ok_or(VoiceErrorCode::ProtocolError)?;
                    let bytes =
                        STANDARD.decode(encoded).map_err(|_| VoiceErrorCode::ProtocolError)?;
                    if bytes.is_empty() || bytes.len() % 2 != 0 {
                        return Err(VoiceErrorCode::ProtocolError);
                    }
                    let position = state.audio.entry(index).or_default();
                    position.0 += bytes.len() as u64 / 2;
                    if position.0 > 24000 * 180 {
                        return Err(VoiceErrorCode::PayloadTooLarge);
                    }
                    for chunk in bytes.chunks(MAX_AUDIO_BYTES) {
                        events.push(VoiceEvent::AudioDelta {
                            response_id: voice_id(state.response.clone())?,
                            item_id: voice_id(item.clone())?,
                            content_index: index,
                            audio: STANDARD.encode(chunk),
                            samples_per_channel: chunk.len() / 2,
                        });
                    }
                } else if kind == "response.output_audio.done" {
                    events.push(VoiceEvent::AudioDone {
                        response_id: voice_id(state.response.clone())?,
                        item_id: voice_id(item)?,
                        content_index: index,
                    });
                } else {
                    let final_text = kind.ends_with(".done") || kind.ends_with(".completed");
                    let field = if final_text { "transcript" } else { "delta" };
                    let text = value[field].as_str().ok_or(VoiceErrorCode::ProtocolError)?;
                    if self.transcripts.len() >= 256
                        && !self.transcripts.contains_key(&(item.clone(), index))
                    {
                        return Err(VoiceErrorCode::Backpressure);
                    }
                    let transcript = self.transcripts.entry((item.clone(), index)).or_default();
                    if transcript.1 {
                        if final_text && transcript.0 != text {
                            return Err(VoiceErrorCode::ProtocolError);
                        }
                        return Ok(vec![]);
                    }
                    if text.len() + if final_text { 0 } else { transcript.0.len() } > 64 * 1024 {
                        return Err(VoiceErrorCode::PayloadTooLarge);
                    }
                    let role = if user { TranscriptRole::User } else { TranscriptRole::Assistant };
                    if final_text {
                        transcript.0 = text.into();
                        transcript.1 = true;
                        events.push(VoiceEvent::TranscriptFinal {
                            role,
                            item_id: voice_id(item)?,
                            content_index: index,
                            text: text.into(),
                        });
                    } else if !transcript.1 {
                        transcript.0.push_str(text);
                        events.push(VoiceEvent::TranscriptDelta {
                            role,
                            item_id: voice_id(item)?,
                            content_index: index,
                            delta: text.into(),
                        });
                    }
                }
            }
            "response.done" => {
                let response = id(&value["response"], "id")?;
                if self.retired.contains(&response) {
                    return Ok(vec![]);
                }
                if self.response.as_deref() != Some(&response) {
                    return Err(VoiceErrorCode::ProtocolError);
                }
                let status = serde_json::from_value(value["response"]["status"].clone())
                    .map_err(|_| VoiceErrorCode::ProtocolError)?;
                self.response = None;
                events.push(VoiceEvent::ResponseDone { response_id: voice_id(response)?, status });
            }
            "error" | "conversation.item.input_audio_transcription.failed" => {
                return Err(VoiceErrorCode::ProviderUnavailable);
            }
            _ => {}
        }
        if self.pending_clear == 0 && self.pending_truncation.is_empty() {
            self.control_deadline = None;
        }
        let mut frames = Vec::new();
        for event in events {
            frames.push(self.frame(event)?);
        }
        Ok(frames)
    }
}
