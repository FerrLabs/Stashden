use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

pub enum Load<T> {
    Loading,
    Failed(String),
    Ready(T),
}

impl<T> Load<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Loading | Self::Failed(_) => None,
        }
    }
}

impl<T, E: std::fmt::Display> From<Result<T, E>> for Load<T> {
    fn from(result: Result<T, E>) -> Self {
        result.map_or_else(|error| Self::Failed(error.to_string()), Self::Ready)
    }
}

pub fn failure(message: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(cx.theme().danger.opacity(0.12))
        .text_color(cx.theme().danger)
        .child(message.into())
}

pub fn notice(message: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(cx.theme().info.opacity(0.12))
        .child(message.into())
}

pub fn hint(message: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(message.into())
}

pub fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

pub fn state(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .py_6()
        .text_center()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}

pub fn row(cx: &App) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .py_1p5()
        .border_b_1()
        .border_color(cx.theme().border)
}

pub fn tag(text: &'static str, danger: bool, cx: &App) -> Div {
    div()
        .px_1p5()
        .rounded_sm()
        .text_xs()
        .bg(if danger {
            cx.theme().danger.opacity(0.15)
        } else {
            cx.theme().secondary
        })
        .child(text)
}
