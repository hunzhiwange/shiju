//! 桌面内嵌字体的测试，覆盖注册顺序、文件签名和字重资产差异。
//! 断言只检查二进制契约，不尝试用等宽字体渲染非 ASCII 文本。
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

#[test]
fn embedded_cjk_font_shapes_and_rasterizes_mixed_ui_text() {
    use iced::advanced::graphics::text::cosmic_text::{
        Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
    };

    // 不扫描系统字体，确认缺少可读取的苹方时也能完成实际字形渲染。
    let mut db = iced::advanced::graphics::text::cosmic_text::fontdb::Database::new();
    db.load_font_data(super::NOTO_SANS_CJK_SC_REGULAR.to_vec());
    db.load_font_data(super::NOTO_SANS_CJK_SC_BOLD.to_vec());
    let mut system = FontSystem::new_with_locale_and_db("zh-CN".into(), db);
    let mut cache = SwashCache::new();
    for weight in [Weight::NORMAL, Weight::BOLD] {
        let mut buffer = Buffer::new(&mut system, Metrics::new(16.0, 24.0));
        buffer.set_size(&mut system, Some(800.0), Some(200.0));
        buffer.set_text(
            &mut system,
            "识矩：选择文件夹开始一个项目，最近项目 shiju /Users/test",
            &Attrs::new().family(Family::Name("Noto Sans CJK SC")).weight(weight),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut system, false);
        let mut count = 0;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                assert_ne!(glyph.glyph_id, 0, "UI text must not use a missing glyph");
                let key = glyph.physical((0.0, 0.0), 1.0).cache_key;
                assert!(cache.get_image(&mut system, key).is_some());
                count += 1;
            }
        }
        assert!(count > 20, "mixed UI text must produce shaped glyphs");
    }
}
