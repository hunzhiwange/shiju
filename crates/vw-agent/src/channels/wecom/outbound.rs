//! Connection-local reply acknowledgements. No queued frame survives a reconnect.

use super::*;
use crate::app::agent::channels::markdown_chunk::split_markdown;
use crate::app::agent::channels::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use crate::app::agent::channels::outbound_format::format_wecom_markdown_tables;
use std::collections::{HashSet, VecDeque};
use tokio::sync::oneshot;

pub(super) const ACK_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_OUTBOUND_ITEMS: usize = 64;
// Conservative local delivery budgets, measured after table alignment and JSON escaping.
const BODY_BYTES: usize = 3000;
const FRAME_BYTES: usize = 16 * 1024;

type Completion = Result<(), DeliveryFailureKind>;

pub(super) fn delivery_error(kind: DeliveryFailureKind) -> anyhow::Error {
    ChannelDeliveryError { confirmed_parts: 0, total_parts: 1, failed_part_index: 0, kind }.into()
}

pub(super) async fn send_reply(
    sender: &mpsc::Sender<OutboundItem>,
    request_id: &str,
    content: &str,
) -> anyhow::Result<()> {
    // Normalize once before splitting: padding a table after splitting could exceed its budget.
    let normalized = format_wecom_markdown_tables(content);
    let chunks = split_markdown(&normalized, BODY_BYTES)
        .map_err(|_| delivery_error(DeliveryFailureKind::NotSent))?;
    let frames: Vec<_> =
        chunks.iter().map(|part| WeComChannel::reply_frame(request_id, part)).collect();
    let total_parts = frames.len();
    for (index, frame) in frames.iter().enumerate() {
        if serde_json::to_vec(frame).map_or(true, |bytes| bytes.len() > FRAME_BYTES) {
            return Err(ChannelDeliveryError {
                confirmed_parts: 0,
                total_parts,
                failed_part_index: index,
                kind: DeliveryFailureKind::NotSent,
            }
            .into());
        }
    }
    for (index, frame) in frames.into_iter().enumerate() {
        let (item, completion) = OutboundItem::new(request_id.to_string(), frame);
        let result = match sender.send(item).await {
            Ok(()) => completion.await.unwrap_or(Err(DeliveryFailureKind::Unknown)),
            Err(_) => Err(DeliveryFailureKind::NotSent),
        };
        if let Err(kind) = result {
            return Err(ChannelDeliveryError {
                confirmed_parts: index,
                total_parts,
                failed_part_index: index,
                kind,
            }
            .into());
        }
    }
    Ok(())
}

pub(super) struct OutboundItem {
    pub request_id: String,
    pub frame: Value,
    completion: Option<oneshot::Sender<Completion>>,
    written: bool,
}

impl OutboundItem {
    pub fn new(request_id: String, frame: Value) -> (Self, oneshot::Receiver<Completion>) {
        let (completion, receiver) = oneshot::channel();
        (Self { request_id, frame, completion: Some(completion), written: false }, receiver)
    }

    fn cancelled(&self) -> bool {
        self.completion.as_ref().is_none_or(oneshot::Sender::is_closed)
    }

    fn complete(mut self, result: Completion) {
        if let Some(completion) = self.completion.take() {
            let _ = completion.send(result);
        }
    }
}

impl Drop for OutboundItem {
    fn drop(&mut self) {
        if let Some(completion) = self.completion.take() {
            let kind = if self.written {
                DeliveryFailureKind::Unknown
            } else {
                DeliveryFailureKind::NotSent
            };
            let _ = completion.send(Err(kind));
        }
    }
}

struct PendingReply {
    item: OutboundItem,
    deadline: Instant,
}

#[derive(Default)]
pub(super) struct OutboundReplies {
    queued: VecDeque<OutboundItem>,
    pending: HashMap<String, PendingReply>,
    // Never reuse an ambiguous req_id on this connection: a late ACK has no sequence number.
    blocked: HashSet<String>,
}

impl OutboundReplies {
    pub fn enqueue(&mut self, item: OutboundItem) {
        if self.blocked.contains(&item.request_id)
            || self.queued.len() + self.pending.len() >= MAX_OUTBOUND_ITEMS
        {
            item.complete(Err(DeliveryFailureKind::NotSent));
        } else if !item.cancelled() {
            self.queued.push_back(item);
        }
    }

    pub fn next_ready(&mut self) -> Option<OutboundItem> {
        self.queued.retain(|item| !item.cancelled());
        let index =
            self.queued.iter().position(|item| !self.pending.contains_key(&item.request_id))?;
        self.queued.remove(index)
    }

    pub fn begin(&mut self, mut item: OutboundItem) {
        // A failed socket write can still have submitted bytes, so it is conservatively Unknown.
        item.written = true;
        self.pending.insert(
            item.request_id.clone(),
            PendingReply { item, deadline: Instant::now() + ACK_TIMEOUT },
        );
    }

    pub fn acknowledge(&mut self, frame: &Value) {
        match frame.get("cmd") {
            None => {}
            Some(Value::String(cmd)) if cmd.is_empty() => {}
            _ => return,
        }
        let Some(request_id) =
            frame.get("headers").and_then(|h| h.get("req_id")).and_then(Value::as_str)
        else {
            return;
        };
        let Some(pending) = self.pending.remove(request_id) else {
            return;
        };
        let result = if pending.deadline <= Instant::now() {
            Err(DeliveryFailureKind::Unknown)
        } else {
            match frame.get("errcode").and_then(Value::as_i64) {
                Some(0) => Ok(()),
                Some(_) => Err(DeliveryFailureKind::Rejected),
                None => Err(DeliveryFailureKind::Unknown),
            }
        };
        if result == Err(DeliveryFailureKind::Unknown) || pending.item.cancelled() {
            self.block(request_id.to_string());
        }
        pending.item.complete(result);
    }

    fn block(&mut self, request_id: String) {
        self.queued.retain(|item| item.request_id != request_id);
        self.blocked.insert(request_id);
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn maintain(&mut self) -> anyhow::Result<()> {
        let expired: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.deadline <= Instant::now() || pending.item.cancelled())
            .map(|(request_id, _)| request_id.clone())
            .collect();
        for request_id in expired {
            // Drop resolves written items as Unknown and unsent queued items as NotSent.
            self.pending.remove(&request_id);
            self.block(request_id);
        }
        self.queued.retain(|item| !item.cancelled());
        if self.blocked.len() >= MAX_OUTBOUND_ITEMS {
            bail!("企微连接存在过多不明回执，停止连接");
        }
        Ok(())
    }
}
