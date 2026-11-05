//! Bounded, single-page PDF rendering using local utilities, without a shell.

#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;

#[cfg(not(target_arch = "wasm32"))]
const MAX_PNG_BYTES: u64 = 8 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
fn select_page(bytes: &[u8], page: u32) -> anyhow::Result<(Vec<u8>, usize)> {
    let mut document = pdf_extract::Document::load_mem(bytes).context("Invalid PDF")?;
    anyhow::ensure!(!document.is_encrypted(), "Encrypted PDFs cannot be rendered");
    let pages = document.get_pages();
    anyhow::ensure!(
        pages.contains_key(&page),
        "Page {page} is out of range ({} pages)",
        pages.len()
    );
    // sips only renders the first page. A single-page PDF also keeps all backends consistent.
    let removed: Vec<u32> = pages.keys().copied().filter(|number| *number != page).collect();
    document.delete_pages(&removed);
    document.prune_objects();
    let mut selected = Vec::new();
    document.save_to(&mut selected)?;
    Ok((selected, pages.len()))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn render_page(bytes: Vec<u8>, page: u32) -> anyhow::Result<String> {
    use base64::Engine as _;
    use std::process::Stdio;
    use std::time::Duration;

    let (selected, total) =
        tokio::task::spawn_blocking(move || select_page(&bytes, page)).await??;
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("page.pdf");
    let output = directory.path().join("page.png");
    tokio::fs::write(&input, selected).await?;

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = tokio::process::Command::new("/usr/bin/sips");
        command.args(["-s", "format", "png", "-Z", "1600"]).arg(&input).arg("--out").arg(&output);
        command
    };
    #[cfg(not(target_os = "macos"))]
    let mut command = {
        let executable = which::which("pdftoppm").context(
            "PDF image rendering requires Poppler (pdftoppm). Install Poppler and add it to PATH",
        )?;
        let mut command = tokio::process::Command::new(executable);
        command
            .args(["-png", "-singlefile", "-scale-to", "1600"])
            .arg(&input)
            .arg(directory.path().join("page"));
        command
    };
    // Discard utility output: only validated image bytes enter the model context.
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true);
    let status = tokio::time::timeout(Duration::from_secs(30), command.status())
        .await
        .context("PDF renderer timed out after 30 seconds")??;
    anyhow::ensure!(status.success(), "PDF renderer exited with {status}");
    let size = tokio::fs::metadata(&output).await.context("Renderer produced no PNG")?.len();
    anyhow::ensure!(
        size > 0 && size <= MAX_PNG_BYTES,
        "Rendered PNG exceeds the 8 MiB limit or is empty"
    );
    let png = tokio::fs::read(&output).await?;
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(&png), image::ImageFormat::Png);
    let (width, height) = reader.into_dimensions().context("Renderer produced invalid PNG")?;
    anyhow::ensure!(
        width > 0 && height > 0 && width <= 1600 && height <= 1600,
        "Rendered PNG dimensions exceed the 1600 pixel limit"
    );
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    Ok(format!(
        "PDF page {page} of {total}; PNG {width}x{height}\n[IMAGE:data:image/png;base64,{encoded}]"
    ))
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn render_page(_bytes: Vec<u8>, _page: u32) -> anyhow::Result<String> {
    anyhow::bail!("PDF image rendering requires the native desktop or CLI runtime")
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "render_tests.rs"]
mod tests;
