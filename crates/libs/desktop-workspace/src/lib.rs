pub mod views {
    pub mod modals { pub mod file_tree_delete; pub mod recent_project_remove; pub mod rename; }
    pub mod project {
        pub mod handles;
        pub mod layout;
        pub mod row_styles;
        pub mod utils;
        pub mod badge_styles;
        pub mod composition;
        pub mod components {
            pub mod session_row;
            pub mod sessions_panel;
            pub mod new_session;
            pub mod projects_list;
        }
    }
}
pub mod components { pub mod project_profiles; }
pub mod project_profiles;
pub mod file_tree;
pub mod basic;
pub mod interaction;
pub mod operations;
pub mod file_manager;

pub mod project_message;

pub mod profile_update;
pub mod projects;

pub mod workspace_state;

pub mod project_edit_view;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_03_tests;

#[cfg(test)]
mod desktop_split_26_tests;

#[cfg(test)]
mod desktop_split_19_tests;
