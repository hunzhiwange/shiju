use super::*;
#[test]
fn voice_ui_dictation_waveform_scrolls_with_audio() {
    let mut waveform = Waveform::default();
    for sequence in 1..=121 {
        waveform.push(sequence, sequence as f32 / 121.0);
    }
    assert_eq!(waveform.samples.len(), 120);
    assert_eq!(waveform.samples.front(), Some(&(2.0 / 121.0)));
    assert_eq!(waveform.samples.back(), Some(&1.0));
    waveform.push(121, 1.0);
    assert_eq!(waveform.samples.back(), Some(&0.5));
    waveform.clear();
    assert!(waveform.samples.is_empty());
    waveform.push(1, 0.75);
    assert_eq!(waveform.samples.back(), Some(&0.75));
}
