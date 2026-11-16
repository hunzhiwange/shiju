//! 会话交互的局部操作，不依赖桌面应用壳。
pub mod message {
    pub mod chat {
        pub mod events;
        pub mod orchestration;
        #[cfg(test)]
        mod orchestration_tests;
        pub mod acp_options;
        pub mod context;
        pub mod fork;
        pub mod session;
        pub mod request;
        pub mod processing_timing;
        pub mod stream;
        pub mod trajectory;
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
pub mod session;
pub mod session_gateway;

pub mod external_open;

pub mod agent_stream;
pub mod agent_stream_events;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_23_tests;

#[cfg(test)]
mod desktop_split_24_tests;

#[cfg(test)]
mod desktop_split_04_tests;

#[cfg(test)]
mod desktop_split_25_tests;

#[cfg(test)]
mod host_split_6_tests;
