//! WebView 初始化脚本生成。

use std::path::Path;

pub fn build_init_script(
    data_path: &Path,
    cookies_json: &str,
    user_agent: &str,
) -> Result<String, serde_json::Error> {
    let data_path_js = serde_json::to_string(&data_path.to_string_lossy())?;
    let ua_js = serde_json::to_string(user_agent)?;
    Ok(format!(
        r#"
        (function() {{
            console.log('%c [VibeWindow] Init Script Starting...', 'color: green; font-weight: bold');
            const dataPath = {data_path_js};
            const userAgent = {ua_js};
            const cookieConfigs = {cookies_json};
            console.log('Data Path:', dataPath);

            const DEFAULT_DOMAIN = '';

            function reportNavigation() {{
                try {{
                    if (window.ipc && typeof window.ipc.postMessage === 'function') {{
                        window.ipc.postMessage(JSON.stringify({{
                            type: 'navigation',
                            url: window.location.href
                        }}));
                    }}
                }} catch (e) {{
                    console.error('Navigation Sync Error:', e);
                }}
            }}

            try {{
                const originalPushState = history.pushState;
                history.pushState = function(...args) {{
                    const result = originalPushState.apply(this, args);
                    reportNavigation();
                    return result;
                }};
                const originalReplaceState = history.replaceState;
                history.replaceState = function(...args) {{
                    const result = originalReplaceState.apply(this, args);
                    reportNavigation();
                    return result;
                }};
                window.addEventListener('popstate', reportNavigation);
                window.addEventListener('hashchange', reportNavigation);
            }} catch (e) {{
                console.error('Navigation Hook Error:', e);
            }}

            function escapeRegExp(s) {{
                return String(s).replace(/[.*+?^${{}}()|[\]\\]/g, '\\$&');
            }}

            function normalizeDomain(domain) {{
                if (!domain) return null;
                const d = String(domain).trim();
                if (!d) return null;
                return d.startsWith('.') ? d : `.${{d}}`;
            }}

            function setCookie(name, value, days, domain) {{
                if (!name) return;
                if (value === undefined || value === null) return;

                const isHttps = window.location && window.location.protocol === 'https:';
                const isHostCookie = String(name).startsWith('__Host-');
                const isSecureCookie = String(name).startsWith('__Secure-') || isHostCookie;

                let maxAge = "";
                if (typeof days === 'number' && isFinite(days) && days > 0) {{
                    maxAge = "; max-age=" + Math.floor(days * 24 * 60 * 60);
                }}

                let serializedCookie = `${{name}}=${{value}}${{maxAge}}; path=/`;

                if (!isHostCookie) {{
                    const d = normalizeDomain(domain || DEFAULT_DOMAIN);
                    if (d) {{
                        serializedCookie += `; domain=${{d}}`;
                    }}
                }}

                if (isHttps || isSecureCookie) {{
                    serializedCookie += "; Secure";
                }}

                document.cookie = serializedCookie;
                const after = getCookie(name);
                if (after !== String(value)) {{
                    console.warn('Set Cookie failed or was rejected:', name);
                }} else {{
                    console.log('Set Cookie:', name);
                }}
            }}

            function getCookie(name) {{
                if (!name) return null;
                const safeName = escapeRegExp(name);
                const match = document.cookie.match(new RegExp('(?:^|;\\s*)' + safeName + '=([^;]+)'));
                return match ? match[1] : null;
            }}

            function initSessionSync() {{
                try {{
                    let configs = cookieConfigs || [];

                    function getBackupName(name) {{
                        return 'vibe_window_webview_backup_' + name;
                    }}

                    configs.forEach(config => {{
                        if (config.url_filter && window.location.href.indexOf(config.url_filter) === -1) {{
                            return;
                        }}

                        const name = config.name;
                        const bName = getBackupName(name);
                        const days = config.days || 365;
                        const domain = config.domain || DEFAULT_DOMAIN;

                        const backupVal = getCookie(bName);
                        const currentVal = getCookie(name);

                        if (backupVal) {{
                            if (!currentVal) {{
                                console.log('Restoring ' + name + ' from ' + bName);
                                setCookie(name, backupVal, days, domain);
                            }} else if (currentVal !== backupVal) {{
                                console.log('Updating ' + name + ' from ' + bName + ' (mismatch detected)...');
                                setCookie(name, backupVal, days, domain);
                            }}
                        }}
                    }});

                    let lastCookie = document.cookie;
                    setInterval(() => {{
                        const currentCookie = document.cookie;
                        if (currentCookie !== lastCookie) {{
                            configs.forEach(config => {{
                                if (config.url_filter && window.location.href.indexOf(config.url_filter) === -1) {{
                                    return;
                                }}

                                const name = config.name;
                                const bName = getBackupName(name);
                                const days = config.days || 365;
                                const domain = config.domain || DEFAULT_DOMAIN;

                                const session = getCookie(name);
                                if (session) {{
                                    const currentBackup = getCookie(bName);
                                    if (session !== currentBackup) {{
                                        console.log('New value for ' + name + ', backing up to ' + bName);
                                        setCookie(bName, session, days, domain);
                                        setCookie(name, session, days, domain);
                                    }}
                                }}
                            }});
                            lastCookie = currentCookie;
                        }}
                    }}, 1000);
                }} catch (e) {{
                    console.error('Session Sync Error:', e);
                }}
            }}

            if (document.readyState === 'loading') {{
                document.addEventListener('DOMContentLoaded', () => {{
                    initSessionSync();
                    reportNavigation();
                }});
            }} else {{
                initSessionSync();
                reportNavigation();
            }}
        }})();
        "#
    ))
}

#[cfg(test)]
mod init_script_tests;
