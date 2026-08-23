// 覆盖配置 trait 的默认实现、异步契约与存储委托。

use super::traits::{ChannelConfig, ConfigHandle};

struct DummyChannel;

impl ChannelConfig for DummyChannel {
    fn name() -> &'static str {
        "Dummy"
    }

    fn desc() -> &'static str {
        "test channel"
    }
}

struct DummyHandle;

impl ConfigHandle for DummyHandle {
    fn name(&self) -> &'static str {
        DummyChannel::name()
    }

    fn desc(&self) -> &'static str {
        DummyChannel::desc()
    }
}

#[test]
fn config_handle_can_delegate_channel_metadata() {
    let handle = DummyHandle;

    assert_eq!(handle.name(), "Dummy");
    assert_eq!(handle.desc(), "test channel");
}
