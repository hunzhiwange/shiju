pub(super) use channel_runtime::dispatch_test_support::{RecordingChannel, context, message};

#[tokio::test]
async fn run_message_dispatch_loop_processes_runtime_commands_until_input_closes() {
    channel_runtime::dispatch_contracts::run_message_dispatch_loop_processes_runtime_commands_until_input_closes(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn run_message_dispatch_loop_limits_in_flight_workers() {
    channel_runtime::dispatch_contracts::run_message_dispatch_loop_limits_in_flight_workers(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel() {
    channel_runtime::dispatch_contracts::run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn run_message_dispatch_loop_keeps_distinct_telegram_senders_independent() {
    channel_runtime::dispatch_contracts::run_message_dispatch_loop_keeps_distinct_telegram_senders_independent(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn run_message_dispatch_loop_stops_when_semaphore_closed_before_receive() {
    channel_runtime::dispatch_contracts::run_message_dispatch_loop_stops_when_semaphore_closed_before_receive(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn run_message_dispatch_loop_does_not_interrupt_model_queries() {
    channel_runtime::dispatch_contracts::run_message_dispatch_loop_does_not_interrupt_model_queries(crate::channels::manager::contract_host()).await;
}
