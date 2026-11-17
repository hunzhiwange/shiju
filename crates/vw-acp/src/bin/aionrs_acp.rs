//! JSON Stream is upstream-only; stdout exposes ACP JSON-RPC exclusively.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if std::env::args_os().len() != 1 {
        eprintln!("aionrs-acp accepts no arguments; configure the model through the environment");
        std::process::exit(2);
    }
    if acp_server::server::serve_with_io(
        acp_server::aionrs::AionrsBackend::default(), tokio::io::stdin(), tokio::io::stdout(),
    ).await.is_err() {
        eprintln!("aionrs ACP connection failed");
        std::process::exit(1);
    }
}
