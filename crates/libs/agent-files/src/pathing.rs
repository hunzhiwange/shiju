use std::path::Path;

/// 检查候选路径是否包含在根目录内。
pub(super) fn contains_path(root: &Path, candidate: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let candidate = if candidate.exists() {
        candidate.canonicalize().unwrap_or_else(|_| candidate.to_path_buf())
    } else {
        let parent = candidate.parent().unwrap_or(root.as_path());
        let base = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
        base.join(candidate.file_name().unwrap_or_default())
    };
    candidate.starts_with(&root)
}

/// 根据文件扩展名判断是否为图片文件。
pub(super) fn is_image_by_extension(filepath: &str) -> bool {
    let ext =
        Path::new(filepath).extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "webp"
            | "ico"
            | "tif"
            | "tiff"
            | "svg"
            | "svgz"
            | "avif"
            | "apng"
            | "jxl"
            | "heic"
            | "heif"
            | "raw"
            | "cr2"
            | "nef"
            | "arw"
            | "dng"
            | "orf"
            | "raf"
            | "pef"
            | "x3f"
    )
}

/// 根据文件扩展名获取图片的 MIME 类型。
pub(super) fn image_mime_type(filepath: &str) -> String {
    let ext =
        Path::new(filepath).extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "tif" | "tiff" => "image/tiff",
        "svg" | "svgz" => "image/svg+xml",
        "avif" => "image/avif",
        "apng" => "image/apng",
        "jxl" => "image/jxl",
        "heic" => "image/heic",
        "heif" => "image/heif",
        _ => return format!("image/{}", ext),
    }
    .to_string()
}

/// 根据文件扩展名判断是否为二进制文件。
pub(super) fn is_binary_by_extension(filepath: &str) -> bool {
    let ext =
        Path::new(filepath).extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "exe"
            | "dll"
            | "pdb"
            | "bin"
            | "so"
            | "dylib"
            | "o"
            | "a"
            | "lib"
            | "wav"
            | "mp3"
            | "ogg"
            | "oga"
            | "ogv"
            | "ogx"
            | "flac"
            | "aac"
            | "wma"
            | "m4a"
            | "weba"
            | "mp4"
            | "avi"
            | "mov"
            | "wmv"
            | "flv"
            | "webm"
            | "mkv"
            | "zip"
            | "tar"
            | "gz"
            | "gzip"
            | "bz"
            | "bz2"
            | "bzip"
            | "bzip2"
            | "7z"
            | "rar"
            | "xz"
            | "lz"
            | "z"
            | "pdf"
            | "doc"
            | "docx"
            | "ppt"
            | "pptx"
            | "xls"
            | "xlsx"
            | "dmg"
            | "iso"
            | "img"
            | "vmdk"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
            | "eot"
            | "sqlite"
            | "db"
            | "mdb"
            | "apk"
            | "ipa"
            | "aab"
            | "xapk"
            | "app"
            | "pkg"
            | "deb"
            | "rpm"
            | "snap"
            | "flatpak"
            | "appimage"
            | "msi"
            | "msp"
            | "jar"
            | "war"
            | "ear"
            | "class"
            | "kotlin_module"
            | "dex"
            | "vdex"
            | "odex"
            | "oat"
            | "art"
            | "wasm"
            | "bc"
            | "ko"
            | "sys"
            | "drv"
            | "efi"
            | "rom"
            | "com"
    )
}
