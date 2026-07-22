use super::*;

#[test]
fn replacement_plan_rejects_empty_or_missing_old_string() {
    let empty = build_replacement_plan("content", "", "new", false).unwrap_err();
    assert!(empty.to_string().contains("must not be empty"));

    let missing = build_replacement_plan("content", "absent", "new", false).unwrap_err();
    assert!(missing.to_string().contains("not found"));
}

#[test]
fn replacement_plan_reports_ambiguous_quote_normalized_matches() {
    let error = build_replacement_plan("‘old’ and 'old'", "\"old\"", "\"new\"", false).unwrap_err();

    assert!(error.to_string().contains("matched 2 locations after quote normalization"));
}

#[test]
fn patch_summary_reports_line_changes_and_hunk_coordinates() {
    let summary = build_patch_summary("notes.txt", "one\ntwo\n", "one\nthree\nfour\n");

    assert_eq!(summary.additions, 2);
    assert_eq!(summary.deletions, 1);
    assert_eq!(summary.hunks.len(), 1);
    let hunk = &summary.hunks[0];
    assert_eq!(hunk.path.as_deref(), Some("notes.txt"));
    assert_eq!(hunk.old_start, Some(1));
    assert_eq!(hunk.new_start, Some(1));
    assert!(hunk.lines.iter().any(|line| line == "-two"));
    assert!(hunk.lines.iter().any(|line| line == "+three"));
}
