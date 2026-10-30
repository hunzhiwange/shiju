//! 有界工具目录分页；拒绝循环游标，不返回不完整目录。
use anyhow::{Result, bail};
use rmcp::{
    Peer, RoleClient,
    model::{PaginatedRequestParam, Tool},
};
use std::{collections::HashSet, time::Duration};

pub(crate) async fn list_tools(peer: &Peer<RoleClient>) -> Result<Vec<Tool>> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut tools = Vec::new();
        let mut cursor = None;
        let mut seen = HashSet::new();
        for _ in 0..100 {
            let page = peer.list_tools(Some(PaginatedRequestParam { cursor })).await?;
            if page.tools.len() > 2048_usize.saturating_sub(tools.len()) {
                bail!("tools/list exceeded the catalog limit of 2048 items");
            }
            tools.extend(page.tools);
            let Some(next) = page.next_cursor else {
                return Ok(tools);
            };
            if next.len() > 64 * 1024 {
                bail!("tools/list pagination cursor exceeds 65536 bytes");
            }
            if !seen.insert(next.clone()) {
                bail!("tools/list returned a repeated pagination cursor");
            }
            cursor = Some(next);
        }
        bail!("tools/list exceeded the pagination limit of 100 pages")
    })
    .await
    .map_err(|_| anyhow::anyhow!("tools/list pagination timed out"))?
}
