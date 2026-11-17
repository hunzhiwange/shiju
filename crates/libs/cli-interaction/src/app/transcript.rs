//! 终端历史与阅读器。

#[cfg(all(not(test), not(target_arch = "wasm32")))]
use super::{
    GatewayV3Bridge,
    Line,
    TerminalMode,
    TranscriptPager,
    V3GatewayApp,
};
#[cfg(any(test, target_arch = "wasm32"))]
use super::{
    Backend,
    CustomTerminal,
    GatewayV3Bridge,
    Line,
    Result,
    TerminalMode,
    TranscriptPager,
    V3GatewayApp,
    Write,
    insert_history_lines,
    io,
};

impl V3GatewayApp {
    /// 返回创建应用时选定的终端模式。
    pub(crate) fn terminal_mode(&self) -> TerminalMode {
        self.terminal_mode
    }

    pub(super) fn command_status_text(&self) -> String {
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        format!(
            "Gateway: {}\nSession: {}\nModel: {}\nPermissions: {}",
            display.gateway_endpoint,
            display.session_id.as_deref().unwrap_or("not created"),
            display.model,
            display.permissions,
        )
    }

    /// 使用聊天视图同一批已提交与活跃单元构造完整 transcript 阅读器。
    pub(crate) fn transcript_pager(&self, _width: u16) -> TranscriptPager {
        TranscriptPager::new(self.transcript_lines(u16::MAX, true))
    }

    /// 按宽度生成 transcript 行，供内联历史插入和窗口缩放回放共用。
    pub(crate) fn transcript_lines(&self, width: u16, include_active: bool) -> Vec<Line<'static>> {
        self.widget.transcript_lines(width, include_active)
    }

    /// 仅将已提交单元写入终端原生回滚区，不泄漏尚未完成的流式内容。
    #[cfg(test)]
    pub(crate) fn insert_committed_history<B>(
        &self,
        terminal: &mut CustomTerminal<B>,
        width: u16,
    ) -> io::Result<()>
    where
        B: Backend<Error = io::Error> + Write,
    {
        insert_history_lines(terminal, self.transcript_lines(width, false))
    }

    /// 仅在宽度确实变化时返回可用于回滚重放的源数据行。
    pub(crate) fn resize_reflow_lines(&mut self, width: u16) -> Option<Vec<Line<'static>>> {
        let change = self.transcript_reflow.note_width(width);
        if change.changed {
            self.transcript_reflow.mark_reflowed_width(width);
            Some(self.transcript_lines(width, true))
        } else {
            None
        }
    }
}
