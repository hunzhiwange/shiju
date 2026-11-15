//! 会话交互的局部操作，不依赖桌面应用壳。
pub mod message {
    pub mod chat {
        pub mod acp_options;
        pub mod context;
        pub mod fork;
        pub mod session;
        pub mod request;
        pub mod processing_timing;
        pub mod stream;
        #[cfg(test)]
        mod processing_timing_tests;
        pub mod input {
            pub mod file_search;
            pub mod shared;
            pub mod slash_palette;
            pub mod task_mode;
            pub mod tool_detail;
            #[cfg(test)]
            mod file_search_tests;
            #[cfg(test)]
            mod shared_tests;
            #[cfg(test)]
            mod slash_palette_tests;
            pub mod editor_actions;
            pub mod clipboard;
            pub mod context_menus;
            pub mod scroll;
            pub mod interactions;
            pub mod popovers;
        }
    }
}
pub mod timeline_identity;
