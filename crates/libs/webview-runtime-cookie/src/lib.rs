//! Trusted runtime session Cookie input and parsing.

use http::{HeaderMap, HeaderValue, header::COOKIE};

const SHIJU_APP_RUNTIME_HOST: &str = "app.shiju.cloud";
const SHIJU_APP_RUNTIME_SESSION_COOKIE: &str = "app_runtime_session";
const MAX_SHIJU_APP_SET_COOKIE_BYTES: usize = 16 * 1024;

fn is_trusted_shiju_app_target(target: &str) -> bool {
    let Some((scheme, remainder)) = target.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("https") {
        return false;
    }

    let Some((authority, path)) = remainder.split_once('/') else {
        return false;
    };
    if !authority.eq_ignore_ascii_case(SHIJU_APP_RUNTIME_HOST) {
        return false;
    }

    let Some(installation_id) = path.strip_prefix("app/").and_then(|path| path.strip_suffix('/'))
    else {
        return false;
    };
    !installation_id.is_empty()
        && installation_id.bytes().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

pub fn read_shiju_app_set_cookie_for_target(
    target: &str,
    input: &mut impl std::io::BufRead,
) -> std::io::Result<String> {
    use std::io::{BufRead, Read};

    if !is_trusted_shiju_app_target(target) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "识矩应用浏览器会话仅支持受信任的运行地址",
        ));
    }

    let mut bytes = Vec::with_capacity(MAX_SHIJU_APP_SET_COOKIE_BYTES + 2);
    input.take((MAX_SHIJU_APP_SET_COOKIE_BYTES + 2) as u64).read_until(b'\n', &mut bytes)?;
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    if bytes.is_empty() || bytes.len() > MAX_SHIJU_APP_SET_COOKIE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "识矩应用浏览器会话为空或超过长度限制",
        ));
    }

    let token = String::from_utf8(bytes).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "识矩应用浏览器会话不是有效 UTF-8")
    })?;
    if token.chars().any(char::is_control) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "识矩应用浏览器会话包含控制字符",
        ));
    }
    Ok(token)
}

pub fn runtime_cookie(set_cookie: &str) -> Result<cookie::Cookie<'static>, &'static str> {
    let mut cookie = cookie::Cookie::parse(set_cookie.to_string())
        .map_err(|_| "识矩应用浏览器会话 Cookie 无效")?;
    if cookie.name() != SHIJU_APP_RUNTIME_SESSION_COOKIE
        || cookie.value().is_empty()
        || cookie.value().contains(';')
        || cookie.value().chars().any(char::is_control)
        || cookie.path() != Some("/")
        || cookie.http_only() != Some(true)
        || cookie.secure() != Some(true)
        || cookie.same_site() != Some(cookie::SameSite::None)
    {
        return Err("识矩应用浏览器会话 Cookie 无效");
    }
    cookie.set_domain(SHIJU_APP_RUNTIME_HOST);
    Ok(cookie)
}

pub fn runtime_cookie_headers(cookie: &cookie::Cookie<'_>) -> Result<HeaderMap, &'static str> {
    let value = HeaderValue::from_str(&format!("{}={}", cookie.name(), cookie.value()))
        .map_err(|_| "识矩应用浏览器会话 Cookie 无效")?;
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, value);
    Ok(headers)
}

#[cfg(test)]
#[path = "cookie_tests.rs"]
mod cookie_tests;
