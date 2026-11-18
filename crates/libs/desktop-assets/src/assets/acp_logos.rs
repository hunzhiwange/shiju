use iced::widget::{image, svg};
use std::collections::HashMap;
use std::sync::LazyLock;

pub enum AcpLogo {
    Monochrome(svg::Handle),
    Color(svg::Handle),
    Raster(image::Handle),
}

static LOGOS: LazyLock<HashMap<&'static str, AcpLogo>> = LazyLock::new(|| {
    macro_rules! svg_logo {
        ($name:literal, $kind:ident) => {
            (
                $name,
                AcpLogo::$kind(svg::Handle::from_memory(include_bytes!(concat!(
                    "../../../../../assets/acp-icons/",
                    $name,
                    ".svg"
                )))),
            )
        };
    }

    HashMap::from([
        svg_logo!("aionrs", Color),
        svg_logo!("codebuddy", Color),
        svg_logo!("amp-acp", Monochrome),
        svg_logo!("autohand", Monochrome),
        svg_logo!("cortex-code", Monochrome),
        svg_logo!("corust-agent", Monochrome),
        svg_logo!("deepagents", Monochrome),
        svg_logo!("devin", Monochrome),
        svg_logo!("dimcode", Monochrome),
        svg_logo!("dirac", Monochrome),
        svg_logo!("glm-acp-agent", Monochrome),
        svg_logo!("goose", Monochrome),
        svg_logo!("grok", Monochrome),
        svg_logo!("harn", Monochrome),
        svg_logo!("junie", Monochrome),
        svg_logo!("mimo-code", Monochrome),
        svg_logo!("nova", Monochrome),
        svg_logo!("omp", Monochrome),
        svg_logo!("poolside", Monochrome),
        svg_logo!("sigit", Monochrome),
        svg_logo!("stakpak", Monochrome),
        svg_logo!("vtcode", Monochrome),
        (
            "snow",
            AcpLogo::Raster(image::Handle::from_bytes(
                include_bytes!("../../../../../assets/acp-icons/snow.png").as_slice(),
            )),
        ),
    ])
});

pub fn get_acp_logo(agent: &str) -> Option<&'static AcpLogo> {
    LOGOS.get(agent)
}
