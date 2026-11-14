//! Pure transcript-to-reader projection, ported from the pinned reader/turns.ts.
use std::collections::{HashMap, HashSet};
use vw_api_types::external_session::{EventKind, SessionBlock, SessionMessage, SessionProvider, StepPhase, ToolKind, ToolStatus, TurnIndex};

#[derive(Debug, Clone)]
pub struct ReaderStep {
    pub id: String,
    pub message_index: usize,
    pub block: SessionBlock,
    pub result: Option<(usize, SessionBlock)>,
    pub body: Option<String>,
    pub cost: Option<SessionBlock>,
    /// Nonempty only for successful Codex read/search or Claude same-server MCP runs.
    pub children: Vec<ReaderStep>,
}
impl ReaderStep {
    pub fn status(&self) -> ToolStatus {
        match self.result.as_ref().map(|(_, b)| b).unwrap_or(&self.block) {
            SessionBlock::ToolResult { status, .. } => *status,
            SessionBlock::ToolCall { id, .. } if id.starts_with("legacy:") => ToolStatus::Unknown,
            SessionBlock::ToolCall { .. } => ToolStatus::Pending,
            _ => ToolStatus::Unknown,
        }
    }
    pub fn is_failure(&self) -> bool { matches!(self.status(), ToolStatus::Error | ToolStatus::Interrupted) }
    pub fn is_change(&self) -> bool {
        self.children.iter().any(Self::is_change) || matches!(&self.block,
            SessionBlock::ToolCall { kind: ToolKind::Edit | ToolKind::Write, .. } |
            SessionBlock::ToolCall { diff: Some(_), .. })
    }
}
#[derive(Debug, Clone)]
pub struct ReaderText {
    pub message_index: usize,
    pub blocks: Vec<SessionBlock>,
    pub ts: Option<i64>,
    pub model: Option<String>,
    pub cost: Option<SessionBlock>,
}
impl ReaderText {
    pub fn text(&self) -> String { join_text(&self.blocks) }
}
#[derive(Debug, Clone)]
pub struct SessionTurn {
    pub key: String,
    pub index: usize,
    pub first_message_index: usize,
    pub last_message_index: usize,
    pub question: Option<ReaderText>,
    pub final_reply: Option<ReaderText>,
    pub leading_events: Vec<ReaderStep>,
    pub steps: Vec<ReaderStep>,
    pub trailing_events: Vec<ReaderStep>,
    pub injected: Vec<usize>,
    pub step_costs: Vec<SessionBlock>,
    pub aborted: bool,
    pub ts: Option<i64>,
    pub end_ts: Option<i64>,
}

pub fn effective_blocks(message: &SessionMessage, index: usize) -> Vec<SessionBlock> {
    if !message.blocks.is_empty() { return message.blocks.clone(); }
    let content = &message.content;
    if content.trim().is_empty() { return Vec::new(); }
    if message.role == "tool" {
        return vec![SessionBlock::ToolResult { call_id: String::new(), status: ToolStatus::Unknown,
            preview: content.clone(), total_len: content.chars().count().min(u32::MAX as usize) as u32,
            line_count: content.lines().count().min(u32::MAX as usize) as u32, truncated: false,
            full: None, exit_code: None, duration_ms: None, images: Vec::new(), saved_path: None }];
    }
    if message.role != "assistant" { return vec![SessionBlock::text(content)]; }
    let mut blocks = Vec::new();
    let mut pending = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let tool = trimmed.strip_prefix("[Tool").and_then(|s| s.split_once(']'))
            .filter(|(name, tail)| (name.is_empty() || name.starts_with(':')) && (tail.is_empty() || tail.starts_with(char::is_whitespace)));
        if let Some((name, title)) = tool {
            flush_text(&mut pending, &mut blocks);
            let name = name.trim_start_matches(':').trim();
            let name = if name.is_empty() { "Tool" } else { name };
            blocks.push(SessionBlock::ToolCall { id: format!("legacy:{index}:{}", blocks.len()), raw_name: name.into(),
                kind: ToolKind::Other, title: if title.trim().is_empty() { name.into() } else { title.trim().into() },
                detail: None, server: None, input_preview: String::new(), input_total_len: 0, input_full: None, diff: None, by_user: false });
        } else { pending.push(line); }
    }
    flush_text(&mut pending, &mut blocks);
    blocks
}
fn flush_text(pending: &mut Vec<&str>, blocks: &mut Vec<SessionBlock>) {
    let text = pending.join("\n");
    if !text.trim().is_empty() { blocks.push(SessionBlock::text(text.trim())); }
    pending.clear();
}
fn join_text(blocks: &[SessionBlock]) -> String {
    blocks.iter().filter_map(|b| match b { SessionBlock::Text { text, .. } if !text.trim().is_empty() => Some(text.as_str()), _ => None }).collect::<Vec<_>>().join("\n\n")
}
fn question(message: &SessionMessage, blocks: &[SessionBlock]) -> bool {
    message.role == "user" && !message.injected && blocks.iter().any(|b| matches!(b, SessionBlock::Image { .. }) || matches!(b, SessionBlock::Text { text, .. } if !text.trim().is_empty()))
}

pub fn project(messages: &[SessionMessage], provider: SessionProvider) -> Vec<SessionTurn> {
    let blocks: Vec<_> = messages.iter().enumerate().map(|(i,m)| effective_blocks(m,i)).collect();
    let mut groups: Vec<(String, usize, usize)> = Vec::new();
    let mut seen = HashMap::<String, usize>::new();
    let mut auto = 0;
    let mut previous = String::new();
    for (i, message) in messages.iter().enumerate() {
        let key = message.turn_id.clone().filter(|v| !v.is_empty()).unwrap_or_else(|| {
            if question(message, &blocks[i]) { auto += 1; }
            format!("auto-{auto}")
        });
        if key == previous && !groups.is_empty() { groups.last_mut().unwrap().2 = i; continue; }
        previous = key.clone();
        let count = seen.entry(key.clone()).or_default();
        let unique = if *count == 0 { key } else { format!("{key}#{count}") };
        *count += 1;
        groups.push((unique, i, i));
    }
    groups.into_iter().enumerate().map(|(index,(key,start,end))| {
        build_turn(messages, &blocks, provider, index, key, start, end)
    }).collect()
}
fn build_turn(messages: &[SessionMessage], blocks: &[Vec<SessionBlock>], provider: SessionProvider, index: usize, key: String, start: usize, end: usize) -> SessionTurn {
    let question_at = (start..=end).find(|&i| question(&messages[i], &blocks[i]));
    let mut last_call = None;
    let mut call_ids = HashSet::new();
    for i in start..=end {
        if messages[i].injected { continue; }
        for (j, block) in blocks[i].iter().enumerate() {
            if let SessionBlock::ToolCall { id, .. } = block { last_call = Some((i,j)); call_ids.insert(id.clone()); }
        }
    }
    let final_at = (start..=end).rev().find_map(|i| {
        if messages[i].role != "assistant" || messages[i].injected || last_call.is_some_and(|(last,_)| i < last) { return None; }
        let from = last_call.filter(|(last,_)| *last == i).map_or(0, |(_,j)| j+1);
        blocks[i][from..].iter().any(|b| matches!(b,SessionBlock::Text {text,..} if !text.trim().is_empty())).then_some((i,from))
    });
    let mut turn = SessionTurn { key, index, first_message_index: start, last_message_index: end,
        question: question_at.map(|i| ReaderText { message_index:i, blocks:blocks[i].iter().filter(|b|matches!(b,SessionBlock::Text{..}|SessionBlock::Image{..})).cloned().collect(),ts:messages[i].ts, model:None,cost:None }),
        final_reply: final_at.map(|(i,from)| ReaderText { message_index:i, blocks:blocks[i][from..].iter().filter(|b|matches!(b,SessionBlock::Text{..}|SessionBlock::Image{..})).cloned().collect(),ts:messages[i].ts,model:messages[i].meta.as_ref().and_then(|m|m.model.clone()),cost:None }),
        leading_events:Vec::new(),steps:Vec::new(),trailing_events:Vec::new(),injected:Vec::new(),step_costs:Vec::new(),aborted:false,ts:None,end_ts:None };
    let mut owners = HashMap::<String, usize>::new();
    let mut early = HashMap::<String,(usize,SessionBlock)>::new();
    let mut segment_start = 0;
    for i in start..=end {
        if let Some(ts) = messages[i].ts { turn.ts.get_or_insert(ts); turn.end_ts=Some(ts); }
        if messages[i].injected { turn.injected.push(i); continue; }
        let mut event_in_message: Option<usize> = None;
        for (j, block) in blocks[i].iter().enumerate() {
            let in_final = final_at.is_some_and(|(at,from)| i==at && j>=from);
            let mut step = ReaderStep { id:format!("{}:{i}:{j}",turn.key),message_index:i,block:block.clone(),result:None,body:None,cost:None,children:Vec::new() };
            match block {
                SessionBlock::Text{text,..} => {
                    if in_final || question_at==Some(i) || text.trim().is_empty() { continue; }
                    if messages[i].role=="system" { if let Some(n) = event_in_message { turn.steps[n].body=Some(text.clone()); continue; } }
                }
                SessionBlock::Image{..} if in_final || question_at==Some(i) => continue,
                SessionBlock::Thinking{text,summary,redacted,..} if text.is_empty() && summary.as_deref().unwrap_or("").is_empty() && !redacted => continue,
                SessionBlock::ToolCall{id,..} => {
                    if !id.is_empty() && !owners.contains_key(id) { owners.insert(id.clone(),turn.steps.len()); step.result=early.remove(id); }
                }
                SessionBlock::ToolResult{call_id,..} => {
                    if let Some(&n)=owners.get(call_id) { if turn.steps[n].result.is_none() { turn.steps[n].result=Some((i,block.clone())); continue; } }
                    else if call_ids.contains(call_id) && !early.contains_key(call_id) { early.insert(call_id.clone(),(i,block.clone())); continue; }
                }
                SessionBlock::Event{kind,..} => {
                    if *kind==EventKind::Aborted { turn.aborted=true; }
                    let leading=question_at.map_or_else(|| !turn.steps.iter().any(|s|!matches!(s.block,SessionBlock::Event{..})), |q|i<=q);
                    if leading { turn.leading_events.push(step); continue; }
                    event_in_message=Some(turn.steps.len());
                }
                SessionBlock::Step{phase,..} => {
                    if *phase==StepPhase::Start { segment_start=turn.steps.len(); continue; }
                    turn.step_costs.push(block.clone());
                    if turn.steps.len()>segment_start { if let Some(last)=turn.steps.last_mut() { last.cost=Some(block.clone()); } }
                    else if let Some(final_reply)=turn.final_reply.as_mut() { final_reply.cost=Some(block.clone()); }
                    segment_start=turn.steps.len(); continue;
                }
                _=>{}
            }
            turn.steps.push(step);
        }
    }
    let trailing=turn.steps.iter().rposition(|s|!matches!(s.block,SessionBlock::Event{..})).map_or(0,|i|i+1);
    turn.trailing_events=turn.steps.split_off(trailing);
    turn.steps=merge_steps(turn.steps,provider);
    turn
}
fn merge_key(step: &ReaderStep, provider: SessionProvider) -> Option<String> {
    if step.status()!=ToolStatus::Success { return None; }
    match &step.block {
        SessionBlock::ToolCall{kind:ToolKind::Read|ToolKind::Search,..} if provider==SessionProvider::Codex => Some("explored".into()),
        SessionBlock::ToolCall{kind:ToolKind::Mcp,server:Some(server),..} if provider==SessionProvider::Claude && !server.is_empty() => Some(format!("mcp:{server}")),
        _=>None,
    }
}
fn merge_steps(steps:Vec<ReaderStep>,provider:SessionProvider)->Vec<ReaderStep> {
    let mut out=Vec::new(); let mut iter=steps.into_iter().peekable();
    while let Some(mut step)=iter.next() {
        if let Some(key)=merge_key(&step,provider) {
            let mut children=Vec::new();
            while iter.peek().is_some_and(|next|merge_key(next,provider).as_ref()==Some(&key)) { children.push(iter.next().unwrap()); }
            if !children.is_empty() { children.insert(0,step.clone()); step.id.push_str("+merged"); step.children=children; }
        }
        out.push(step);
    }
    out
}
/// Compact directory excludes pure injected/event-only turns except slash commands.
pub fn build_turn_index(turns:&[SessionTurn])->Vec<TurnIndex> {
    turns.iter().filter_map(|turn| {
        let slash=turn.leading_events.iter().find_map(|s|match &s.block {SessionBlock::Event{kind:EventKind::SlashCommand,text,..}=>text.clone(),_=>None});
        if turn.question.is_none() && slash.is_none() { return None; }
        let source=turn.question.as_ref().map(ReaderText::text).filter(|s|!s.is_empty()).or(slash).unwrap_or_default();
        let flat=source.split_whitespace().collect::<Vec<_>>().join(" ");
        let preview=if flat.chars().count()>80 { format!("{}…",flat.chars().take(80).collect::<String>()) } else {flat};
        let count=|failure:bool| turn.steps.iter().flat_map(|s|if s.children.is_empty(){vec![s]}else{s.children.iter().collect()}).filter(|s|matches!(s.block,SessionBlock::ToolCall{..}|SessionBlock::ToolResult{..}) && (!failure||s.is_failure())).count().min(u32::MAX as usize) as u32;
        Some(TurnIndex{turn_id:turn.key.clone(),first_message_index:turn.first_message_index,last_message_index:turn.last_message_index,question_preview:preview,ts:turn.question.as_ref().and_then(|q|q.ts).or(turn.ts),step_count:count(false),error_count:count(true),has_final_reply:turn.final_reply.is_some(),aborted:turn.aborted})
    }).collect()
}
/// Compact projection for streaming headers; merging does not change directory counts.
pub fn build_turns(messages:&[SessionMessage])->Vec<TurnIndex> { build_turn_index(&project(messages,SessionProvider::Codex)) }

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ReaderFilter { All, Conversation, Changes }
#[derive(Debug,Clone,PartialEq,Eq)]
pub enum ReaderRow { Divider(usize), Question(usize), Final(usize), Step(usize,String), Event(usize,String), Injected(usize,usize) }
/// Filtered rows retain stable turn/step keys; injection visibility is explicit.
pub fn flatten_rows(turns:&[SessionTurn],filter:ReaderFilter,show_injected:bool)->Vec<ReaderRow> {
    let mut rows=Vec::new();
    for turn in turns {
        let mut own=Vec::new();
        let changes:Vec<_>=turn.steps.iter().filter(|s|filter!=ReaderFilter::Changes||s.is_change()).collect();
        if filter==ReaderFilter::Changes && changes.is_empty(){continue;}
        if filter==ReaderFilter::All {
            own.extend(turn.leading_events.iter().map(|s|ReaderRow::Event(turn.index,s.id.clone())));
            if show_injected {own.extend(turn.injected.iter().map(|i|ReaderRow::Injected(turn.index,*i)));}
        }
        if turn.question.is_some(){own.push(ReaderRow::Question(turn.index));}
        if filter!=ReaderFilter::Conversation {own.extend(changes.iter().map(|s|ReaderRow::Step(turn.index,s.id.clone())));}
        if filter!=ReaderFilter::Changes && turn.final_reply.is_some(){own.push(ReaderRow::Final(turn.index));}
        if filter==ReaderFilter::All {own.extend(turn.trailing_events.iter().map(|s|ReaderRow::Event(turn.index,s.id.clone())));}
        if !own.is_empty(){if !rows.is_empty(){rows.push(ReaderRow::Divider(turn.index));} rows.extend(own);}
    }
    rows
}
