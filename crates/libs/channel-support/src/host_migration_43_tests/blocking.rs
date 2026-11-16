use crate::blocking::block_on_future;
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_future_runs_outside_runtime() {
    // 不在 Tokio 运行时中调用时，应自建运行时执行 Future。
    let value = block_on_future(async { 7_u32 + 35 });
    assert_eq!(value, 42);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_future_runs_inside_runtime() {
    // 已处于运行时中时，走 block_in_place 分支。
    let rt =
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let value = rt.block_on(async { block_on_future(async { "ok" }) });
    assert_eq!(value, "ok");
}
