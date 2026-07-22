# SVG

[Svg](https://docs.rs/iced/0.13.1/iced/widget/svg/struct.Svg.html) 组件用于显示 SVG 图片。
要使用该组件，需要启用 [svg](https://docs.rs/crate/iced/0.13.1/features#svg) 特性。

```toml
[dependencies]
iced = { version = "0.13.1", features = ["svg"] }
```

SVG 组件支持两种图片来源：[Handle](https://docs.rs/iced/0.13.1/iced/widget/svg/struct.Handle.html) 和 [SvgData](https://docs.rs/iced/0.13.1/iced/widget/svg/struct.SvgData.html)。

### Handle

[Handle](https://docs.rs/iced/0.13.1/iced/widget/svg/struct.Handle.html) 用于从路径、URL 或内存中的 bytes 加载 SVG。

```rust
use iced::{
    widget::svg,
    widget::svg::Handle,
};

fn main() -> iced::Result {
    let svg1 = Svg::<()>::from(Handle::from_path("images/foo.svg"));
    let svg2 = Svg::<()>::from(Handle::from_url(
        "https://example.com/foo.svg",
    ));
    let bytes = include_bytes!("../foo.svg");
    let svg3 = Svg::<()>::from(Handle::from_memory(bytes));
}
```

### SvgData

[SvgData](https://docs.rs/iced/0.13.1/iced/widget/svg/struct.SvgData.html) 用于从 SVG 字符串创建 SVG。

```rust
use iced::widget::svg::SvgData;

fn main() -> iced::Result {
    let svg_data = SvgData::from_bytes(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="40" fill="red"/>
        </svg>"#
    );
    let svg = Svg::<()>::from(svg_data);
}
```

:arrow_right: 下一篇: [图片](./image.md)

:blue_book: 返回: [目录](./../README.md)