use super::patch_toml::{PatchError, TomlPatch};
use toml_edit::{DocumentMut, Item, value};

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

#[test]
fn cc04_patch_toml_preserves_comments_and_quoted_keys() {
    let source = "# 中文🙂\n\"model.name\" = 'old' # keep\nagent.model = \"old\" # dotted\ninline = { \"a.b\" = 1, other = 'stay' } # inline\n\n[untouched]\nnumber = 0xDEAD # exact\n";
    let patch = TomlPatch {
        set: vec![
            (path(&["model.name"]), value("new")),
            (path(&["agent", "model"]), value("new")),
            (path(&["inline", "a.b"]), value(2)),
        ],
        ..Default::default()
    };
    let output = patch.apply(Some(source.as_bytes())).unwrap();
    let expected =
        source.replace("'old'", "\"new\"").replace("\"old\"", "\"new\"").replace("= 1,", "= 2,");
    assert_eq!(String::from_utf8(output.clone()).unwrap(), expected);
    assert_eq!(patch.apply(Some(&output)).unwrap(), output);
}

#[test]
fn cc04_patch_toml_invalid_document_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config.toml");
    let source = b"token = 'fixture-secret'\nbroken = [\n";
    std::fs::write(&file, source).unwrap();
    let patch = TomlPatch { set: vec![(path(&["token"]), value("new"))], ..Default::default() };
    assert_eq!(patch.apply(Some(&std::fs::read(&file).unwrap())), Err(PatchError::Parse));
    assert_eq!(std::fs::read(file).unwrap(), source);
}

#[test]
fn cc04_patch_toml_adds_tables_inside_inline_and_preserves_other_tables() {
    let fragment: DocumentMut = "name = 'new'\n[child]\nx = 1\n".parse().unwrap();
    let patch = TomlPatch {
        set: vec![
            (path(&["inline", "nested", "provider"]), Item::Table(fragment.as_table().clone())),
            (path(&["providers", "a.b"]), Item::Table(fragment.as_table().clone())),
        ],
        ..Default::default()
    };
    let source = "inline = { keep = 1 } # keep\n\n[ui]\ntheme = 'dark' # user\n";
    let output = String::from_utf8(patch.apply(Some(source.as_bytes())).unwrap()).unwrap();
    assert!(output.contains("[ui]\ntheme = 'dark' # user\n"));
    assert!(output.contains("# keep"));
    let parsed: toml::Table = toml::from_str(&output).unwrap();
    assert_eq!(parsed["inline"]["nested"]["provider"]["child"]["x"].as_integer(), Some(1));
    assert_eq!(parsed["providers"]["a.b"]["name"].as_str(), Some("new"));
}

#[test]
fn cc04_patch_toml_remove_seed_and_reject_partial_batch() {
    let patch = TomlPatch {
        remove: vec![path(&["inline", "old"]), path(&["keep"]), path(&["table"])],
        set: vec![(path(&["keep"]), value(3))],
        seed: vec![(path(&["keep"]), value(9)), (path(&["new", "key"]), value(true))],
    };
    let output = patch
        .apply(Some(b"keep = 1 # stay\ninline = { old = 2, other = 4 }\n[table]\nx = 1\n"))
        .unwrap();
    let doc: toml::Table = toml::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    assert_eq!(doc["keep"].as_integer(), Some(3));
    assert!(doc["inline"].get("old").is_none());
    assert!(doc.get("table").is_none());
    assert_eq!(doc["new"]["key"].as_bool(), Some(true));
    let bad = TomlPatch {
        set: vec![(path(&["keep"]), value(5)), (path(&["inline", "other", "x"]), value(1))],
        ..Default::default()
    };
    assert_eq!(bad.apply(Some(&output)), Err(PatchError::Shape));
    assert_eq!(
        TomlPatch { remove: vec![vec![]], ..Default::default() }.apply(None),
        Err(PatchError::InvalidPath)
    );
    assert_eq!(TomlPatch::default().apply(Some(&[0xff])), Err(PatchError::Parse));
    assert!(TomlPatch::default().apply(None).unwrap().is_empty());
}
