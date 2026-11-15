//! 会话交互的局部操作，不依赖桌面应用壳。
pub mod message {
    pub mod chat {
        pub mod acp_options;
        pub mod context;
        pub mod input {
            pub mod editor_actions;
            pub mod clipboard;
            pub mod context_menus;
            pub mod scroll;
        }
    }
}
pub mod timeline_identity;
