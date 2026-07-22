use std::borrow::Cow;

#[test]
fn load_all_returns_borrowed_fonts_in_registration_order() {
    let expected = [
        super::JETBRAINS_MONO_REGULAR,
        super::JETBRAINS_MONO_BOLD,
        super::NOTO_SANS_CJK_SC_REGULAR,
        super::NOTO_SANS_CJK_SC_BOLD,
    ];

    let fonts = super::load_all();

    assert_eq!(fonts.len(), expected.len());
    for (font, expected_bytes) in fonts.into_iter().zip(expected) {
        let Cow::Borrowed(actual_bytes) = font else {
            panic!("embedded fonts must remain borrowed");
        };
        assert_eq!(actual_bytes, expected_bytes);
    }
}

#[test]
fn embedded_font_assets_have_expected_sfnt_signatures() {
    let fonts = [
        ("JetBrains Mono Regular", super::JETBRAINS_MONO_REGULAR, b"\0\x01\0\0".as_slice()),
        ("JetBrains Mono Bold", super::JETBRAINS_MONO_BOLD, b"\0\x01\0\0".as_slice()),
        ("Noto Sans CJK SC Regular", super::NOTO_SANS_CJK_SC_REGULAR, b"OTTO".as_slice()),
        ("Noto Sans CJK SC Bold", super::NOTO_SANS_CJK_SC_BOLD, b"OTTO".as_slice()),
    ];

    for (name, bytes, signature) in fonts {
        assert!(bytes.len() > signature.len(), "{name} must contain font data");
        assert_eq!(&bytes[..signature.len()], signature, "unexpected signature for {name}");
    }
}

#[test]
fn regular_and_bold_font_assets_are_distinct() {
    assert!(
        super::JETBRAINS_MONO_REGULAR != super::JETBRAINS_MONO_BOLD,
        "JetBrains Mono weights must use distinct assets"
    );
    assert!(
        super::NOTO_SANS_CJK_SC_REGULAR != super::NOTO_SANS_CJK_SC_BOLD,
        "Noto Sans CJK SC weights must use distinct assets"
    );
}
