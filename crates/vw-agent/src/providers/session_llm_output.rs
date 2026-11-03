//! Split the gateway's reserved think delimiters before exposing provider text.

#[derive(Default)]
pub(super) struct GatewayOutput {
    pending: String,
    reasoning: String,
    in_reasoning: bool,
}

impl GatewayOutput {
    pub(super) fn push(&mut self, delta: &str) -> String {
        self.pending.push_str(delta);
        let mut visible = String::new();
        loop {
            let tag = if self.in_reasoning { "</think>" } else { "<think>" };
            if let Some(index) = self.pending.find(tag) {
                let prefix: String = self.pending.drain(..index).collect();
                if self.in_reasoning {
                    self.reasoning.push_str(&prefix);
                } else {
                    visible.push_str(&prefix);
                }
                self.pending.drain(..tag.len());
                self.in_reasoning = !self.in_reasoning;
                continue;
            }
            // Only retain a possible delimiter suffix, including across UTF-8 chunks.
            let keep = (1..tag.len())
                .rev()
                .find(|length| self.pending.ends_with(&tag[..*length]))
                .unwrap_or(0);
            let prefix: String = self.pending.drain(..self.pending.len() - keep).collect();
            if self.in_reasoning {
                self.reasoning.push_str(&prefix);
            } else {
                visible.push_str(&prefix);
            }
            return visible;
        }
    }

    pub(super) fn finish(&mut self) -> Result<(String, Option<String>), String> {
        if self.in_reasoning {
            return Err("gateway reasoning block was not closed".to_string());
        }
        let reasoning = std::mem::take(&mut self.reasoning);
        Ok((std::mem::take(&mut self.pending), (!reasoning.is_empty()).then_some(reasoning)))
    }
}
