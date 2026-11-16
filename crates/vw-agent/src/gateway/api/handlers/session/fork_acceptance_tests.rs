use super::message_ops::Host;
#[test]
#[should_panic(expected = "discarded history leaked: fork-discarded-second")]
fn fork_acceptance_negative_detects_discarded_model_history() {
    gateway_session::native_fork_acceptance::fork_acceptance_negative_detects_discarded_model_history();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_acceptance_restarts_and_resumes_both_targets() {
    let test = concat!(module_path!(), "::fork_acceptance_restarts_and_resumes_both_targets");
    gateway_session::native_fork_acceptance::fork_acceptance_restarts_and_resumes_both_targets::<Host>(test.split_once("::").unwrap().1).await;
}
