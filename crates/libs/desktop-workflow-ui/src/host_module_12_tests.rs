//! Original workflow controller assertions using only domain state.
#![allow(unused_must_use)]
use crate::controller::*;
use crate::controller::{app, document, run};
use crate::message::{WorkflowMessage, WorkflowSaveTarget};
use desktop_workflow_model::*;
use desktop_workflow_state::state::*;
use iced::{Point, widget::text_editor};
struct Fixture {
    workflow_state: WorkflowState,
    window_size: (f32, f32),
}
impl Fixture {
    fn new() -> (Self, ()) {
        (Self { workflow_state: WorkflowState::default(), window_size: (1200.0, 800.0) }, ())
    }
    fn context(&mut self) -> Context<'_> {
        Context {
            workflow_state: &mut self.workflow_state,
            window_size: self.window_size,
            tenant: vw_api_transport::TenantScope::new("10000", "test-company").unwrap(),
            gateway: || Err("offline fixture".into()),
            sync_top_tab: false,
        }
    }
}
fn sync_video_tenant(app: &mut Fixture) {
    let context = app.context();
    context.workflow_state.select_tenant(context.tenant.clone());
}

#[path = "host_module_12_tests/app_tests.rs"]
mod app_tests;

#[path = "host_module_12_tests/document_tests.rs"]
mod document_tests;

#[path = "host_module_12_tests/run_tests.rs"]
mod run_tests;

#[path = "host_module_12_tests/mod_tests.rs"]
mod mod_tests;

#[path = "host_module_12_tests/tests.rs"]
mod tests;

#[path = "host_module_12_tests/video_edit_tests.rs"]
mod video_edit_tests;
