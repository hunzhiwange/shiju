use super::memory_update::normalize_backend;

#[test]
fn normalize_backend_accepts_supported_values_and_normalizes_aliases() {
    for backend in ["sqlite", "postgres", "qdrant", "chroma", "markdown", "none"] {
        assert_eq!(normalize_backend(backend), backend);
    }

    assert_eq!(normalize_backend("  QDRANT  "), "qdrant");
    assert_eq!(normalize_backend(" NULL "), "none");
    assert_eq!(normalize_backend("unknown"), "sqlite");
    assert_eq!(normalize_backend("  "), "sqlite");
}
