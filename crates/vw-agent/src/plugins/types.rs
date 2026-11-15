//! 插件清单类型的统一入口；实现由 plugin-types 持有。

pub use plugin_types::{
    PluginAuthor, PluginAuthorDetails, PluginHookSource, PluginIdentity, PluginInstallMetadata,
    PluginInterface, PluginManifest, PluginMcpServers, PluginPath,
};
