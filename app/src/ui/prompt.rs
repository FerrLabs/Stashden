use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;

use super::parts;

pub struct Ask {
    pub heading: String,
    pub label: &'static str,
    pub hint: Option<&'static str>,
    pub value: String,
    pub action: &'static str,
}

pub fn ask(
    question: Ask,
    window: &mut Window,
    cx: &mut App,
    answered: impl Fn(String, &mut Window, &mut App) + 'static,
) {
    let value = question.value.clone();
    let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
    let first = Cell::new(true);
    let answered = Rc::new(answered);
    window.open_alert_dialog(cx, move |alert, window, cx| {
        if first.replace(false) {
            input.focus_handle(cx).focus(window, cx);
        }
        let input_for_ok = input.clone();
        let answered = Rc::clone(&answered);
        alert
            .title(question.heading.clone())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().child(question.label))
                    .child(Input::new(&input))
                    .children(question.hint.map(|hint| parts::hint(hint, cx))),
            )
            .confirm()
            .ok_text(question.action)
            .on_ok(move |_, window, cx| {
                let typed = input_for_ok.read(cx).value().trim().to_owned();
                if typed.is_empty() {
                    return false;
                }
                answered(typed, window, cx);
                true
            })
    });
}

pub fn confirm(
    heading: String,
    body: String,
    action: &'static str,
    window: &mut Window,
    cx: &mut App,
    accepted: impl Fn(&mut Window, &mut App) + 'static,
) {
    let accepted = Rc::new(accepted);
    window.open_alert_dialog(cx, move |alert, _, _| {
        let accepted = Rc::clone(&accepted);
        alert
            .title(heading.clone())
            .description(body.clone())
            .confirm()
            .ok_text(action)
            .ok_variant(ButtonVariant::Danger)
            .on_ok(move |_, window, cx| {
                accepted(window, cx);
                true
            })
    });
}
