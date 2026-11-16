use super::*;
use std::fs;

#[test]
fn shiju_download_writer_preserves_chunk_order_and_propagates_errors() {
    let archive = NamedTempFile::new().expect("archive fixture");
    let archive_path = archive.path().to_path_buf();
    let (sender, receiver) = tokio::sync::mpsc::channel(4);
    sender.try_send(ShijuDownloadChunk::Data(b"first-".to_vec())).expect("queue first chunk");
    sender.try_send(ShijuDownloadChunk::Data(b"second".to_vec())).expect("queue second chunk");
    sender.try_send(ShijuDownloadChunk::Complete).expect("queue completion");
    drop(sender);

    let archive =
        write_shiju_download_chunks(archive, receiver).expect("write queued download chunks");
    assert_eq!(fs::read(&archive_path).expect("read archive fixture"), b"first-second");
    drop(archive);

    let archive = NamedTempFile::new().expect("error archive fixture");
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    sender.try_send(ShijuDownloadChunk::Data(b"partial".to_vec())).expect("queue partial chunk");
    sender.try_send(ShijuDownloadChunk::Failed("下载中断".into())).expect("queue download error");
    drop(sender);

    let error = write_shiju_download_chunks(archive, receiver)
        .expect_err("download error must stop the writer");
    assert_eq!(error, "下载中断");

    let archive = NamedTempFile::new().expect("cancelled archive fixture");
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    drop(sender);
    let error = write_shiju_download_chunks(archive, receiver)
        .expect_err("closed channel must not look complete");
    assert_eq!(error, "识矩技能安装包下载未完成");
}
