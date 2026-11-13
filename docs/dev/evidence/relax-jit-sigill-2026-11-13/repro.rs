use chrono::{Local,TimeZone};
#[inline(never)]
fn local_time(timestamp: u64) -> String {
    i64::try_from(timestamp).ok().filter(|timestamp| *timestamp > 0)
        .and_then(|timestamp| Local.timestamp_millis_opt(timestamp).single())
        .map_or_else(|| "—".to_owned(), |time| time.format("%Y-%m-%d %H:%M").to_string())
}
fn main() {
    for _ in 0..200 { std::thread::sleep(std::time::Duration::from_millis(50)); for _ in 0..1000 { assert_eq!(local_time(0), "—"); assert!(!local_time(1794531600000).is_empty()); } }
    println!("time repro passed");
}
