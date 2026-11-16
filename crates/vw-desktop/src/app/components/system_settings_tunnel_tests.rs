use crate::app::{App, Message};
use crate::app::message::settings::{SettingsMessage, TunnelMessage};
use crate::app::i18n::settings_extensions::text as tr;
fn test_app() -> App { App::new().0 }
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_17_renders_contextual_help() {
    use super::super::{
        system_settings::SystemTab,
        system_settings_network as network,
        system_settings_proxy::system_settings_proxy_tests::rollout::{
            MODES, Scene, isolated, renderer,
        },
    };

    if !isolated(concat!(module_path!(), "::help_rollout_17_renders_contextual_help")) {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = test_app();
    app.navigation_panels.system_settings_tab = SystemTab::Tunnel;
    app.tunnel_settings.save_error = Some("Fixture tunnel connection failed".into());
    app.tunnel_settings.custom_start_command = "fixture-tunnel {port}".into();

    for (locale, theme) in MODES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| tr(locale, key);
        for (provider, fields) in [
            (
                "custom",
                &[
                    ("启动命令", "支持在命令中使用 {host} 与 {port} 占位符。"),
                    ("健康检查 URL", "可选，用于确认隧道服务是否已准备就绪。"),
                    ("URL 模式", "可选，可填前缀或正则片段用于提取公网地址。"),
                    (
                        "自定义隧道",
                        "当前运行时使用命令型自定义隧道，因此这里配置 start_command / health_url / url_pattern。",
                    ),
                ][..],
            ),
            (
                "cloudflare",
                &[
                    ("Cloudflare Tunnel", "使用 cloudflared token 将本地网关安全暴露到公网。"),
                    ("令牌", "Zero Trust 侧下发的 tunnel token。"),
                ][..],
            ),
            ("tailscale", &[("主机名", "留空时将使用当前 Tailscale 节点名。")][..]),
            (
                "ngrok",
                &[
                    ("ngrok", "配置 ngrok token，并可选绑定自定义 domain。"),
                    ("认证令牌", "ngrok 控制台生成的 auth token。"),
                    ("域名", "可选，绑定预留的自定义域名。"),
                ][..],
            ),
            ("none", &[("未启用隧道", "保持 provider=none 时，网关不会自动创建公网隧道。")][..]),
        ] {
            app.tunnel_settings.provider = provider.into();
            let mut scene = Scene::new(&app, &mut renderer, provider, network::view(&app));
            scene.label("Fixture tunnel connection failed");
            for (title, description) in fields {
                scene.help(t(title), t(description));
            }
            if provider == "custom" {
                scene.help(t("接入方式"), t("选择隧道 provider，并根据 provider 展开对应参数。"));
                scene.label(t("选择自动暴露网关到公网或私有网络的方式。"));
                assert!(matches!(
                    scene.edit("fixture-tunnel {port}").as_slice(),
                    [Message::Settings(SettingsMessage::Tunnel(TunnelMessage::CustomStartCommandChanged(value)))]
                        if value == "fixture-tunnel {port"
                ));
            }
            if provider == "tailscale" {
                scene.label(t("serve 仅 tailnet 内可访问，funnel 才会暴露到公网。访问地址以启动日志为准，通常不要使用本地监听端口。"));
                let checkbox = scene.label(t("启用 funnel（关闭时使用 serve）")).center();
                assert!(matches!(
                    scene.click(checkbox).as_slice(),
                    [Message::Settings(SettingsMessage::Tunnel(
                        TunnelMessage::TailscaleFunnelToggled(true)
                    ))]
                ));
            }
            if provider == "none" {
                scene.label(t("当前仅监听本地网关地址，不会主动创建公网或 tailnet 暴露。"));
            }
        }
    }
}
