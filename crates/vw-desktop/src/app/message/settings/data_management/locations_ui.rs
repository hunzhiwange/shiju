//! Category presentation and client-local reveal boundary.
use vw_gateway_client::{DataBackupLocation, DataBackupLocationDeletion};

pub(crate) fn can_cleanup(location: &DataBackupLocation) -> bool {
    location.id != "database" && location.deletable
}
pub(crate) fn can_reveal(loopback: bool, host: &str) -> bool {
    loopback && host == vw_shared::local_apps::HOST_ID
}
pub(crate) fn cleanup_summary(report: &DataBackupLocationDeletion) -> String {
    if report.undeleted_count > 0 {
        format!("部分清理：已释放 {} 字节，{} 项未删除。请核对保护规则后刷新。", report.freed_bytes, report.undeleted_count)
    } else { format!("清理完成：已释放 {} 字节。", report.freed_bytes) }
}

#[cfg(test)]
#[path = "locations_ui_tests.rs"]
mod tests;
