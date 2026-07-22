# Button（按钮）

[Button](https://docs.rs/iced/0.13.1/iced/widget/button/struct.Button.html) 组件支持对按下/触摸事件的响应。
有两种构造方式：`button` 函数和 `Button::new` 构造器。
默认情况下，若未定义 [on_press](https://docs.rs/iced/0.13.1/iced/widget/button/struct.Button.html#method.on_press)，按钮处于禁用状态。
我们还可以设置按钮文本周围的内边距。

```rust
use iced::widget::{Button, button, column};

fn main() -> iced::Result {
    iced::run("My App", MyApp::update, MyApp::view)
}

#[derive(Debug, Clone)]
enum Message {
    DoSomething,
}

#[derive(Default)]
struct MyApp;

impl MyApp {
    fn update(&mut self, _message: Message) {}

    fn view(&self) -> iced::Element<Message> {
        column![
            Button::new("Disabled button"),
            button("Construct from function"),
            button("Enabled button").on_press(Message::DoSomething),
            button("With padding").padding(20),
        ]
        .into()
    }
}
```

![Button](./pic/button.png)

:arrow_right: 下一篇：[TextInput](./text_input.md)

:blue_book: 返回：[目录](./../README.md)
