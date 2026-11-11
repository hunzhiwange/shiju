use super::{capture::device_lease, playback::fixture, preview::Preview};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use vw_config_types::voice::VoiceName;
use vw_gateway_client::GatewayEndpoint;

#[test]
fn voice_task_13_voice_preview_does_not_capture_microphone() {
    let lease = device_lease();
    assert!(!lease.swap(true, Ordering::AcqRel));
    struct Lease(Arc<AtomicBool>);
    impl Drop for Lease {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _lease = Lease(lease.clone());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut request = [0; 8192];
        let _ = stream.read(&mut request).unwrap();
        let audio: Vec<u8> = [16384i16; 480].into_iter().flat_map(i16::to_le_bytes).collect();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", audio.len()).unwrap();
        stream.write_all(&audio).unwrap();
    });
    let (playback, mut output) = fixture(24000, 1);
    let heard = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let callback_heard = heard.clone();
    let callback_done = done.clone();
    let callback = std::thread::spawn(move || {
        while !callback_done.load(Ordering::Acquire) {
            let mut pcm = [0.0f32; 480];
            output.render(&mut pcm);
            if pcm.contains(&0.5) {
                callback_heard.store(true, Ordering::Release);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    let preview = Preview::start_output(endpoint, VoiceName::Marin, || Ok(playback)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let result = loop {
        if let Some(result) = preview.result() {
            break result;
        }
        assert!(
            Instant::now() < deadline,
            "preview must finish with the microphone already leased"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    drop(preview);
    done.store(true, Ordering::Release);
    callback.join().unwrap();
    server.join().unwrap();
    assert!(result.is_ok(), "{result:?}");
    assert!(heard.load(Ordering::Acquire), "actual playback callback must emit the fixture PCM");
    assert!(lease.load(Ordering::Acquire), "preview must not acquire or release the microphone");
}
