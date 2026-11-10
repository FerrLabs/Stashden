use std::path::Path;
use std::sync::Arc;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::parts;
use super::run;
use crate::failure::Failure;
use crate::update::{self, CURRENT, Offered};

enum Looking {
    Checking,
    Latest,
    Available(Arc<Offered>),
    Failed(String),
}

pub struct UpdateDialog {
    looking: Looking,
    installing: bool,
    failure: Option<String>,
}

pub fn open(window: &mut Window, cx: &mut App) {
    let dialog = cx.new(|cx| UpdateDialog::new(window, cx));
    window.open_dialog(cx, move |shell, _, _| {
        shell.title("Updates").w(px(460.)).child(dialog.clone())
    });
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, Failure> + Send + 'static,
) -> Result<T, Failure> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| Failure::from(error.to_string()))?
}

impl UpdateDialog {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        run(
            window,
            cx,
            blocking(update::check),
            |this, checked, _, cx| {
                this.looking = match checked {
                    Ok(Some(offered)) => Looking::Available(Arc::new(offered)),
                    Ok(None) => Looking::Latest,
                    Err(failure) => Looking::Failed(failure.to_string()),
                };
                cx.notify();
            },
        );
        Self {
            looking: Looking::Checking,
            installing: false,
            failure: None,
        }
    }

    fn install(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Looking::Available(offered) = &self.looking else {
            return;
        };
        let offered = Arc::clone(offered);
        self.installing = true;
        self.failure = None;
        cx.notify();
        run(
            window,
            cx,
            blocking(move || update::install(&offered)),
            |this, installed, _, cx| match installed {
                Ok(path) => {
                    relaunch(&path);
                    cx.quit();
                }
                Err(failure) => {
                    this.installing = false;
                    this.failure = Some(format!("The update did not install: {failure}"));
                    cx.notify();
                }
            },
        );
    }
}

#[cfg(target_os = "macos")]
fn relaunch(path: &Path) {
    let _ = std::process::Command::new("open")
        .arg("-n")
        .arg(path)
        .spawn();
}

#[cfg(not(target_os = "macos"))]
fn relaunch(path: &Path) {
    let _ = std::process::Command::new(path).spawn();
}

impl Render for UpdateDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.looking {
            Looking::Checking => parts::hint("Looking for a new version", cx),
            Looking::Latest => {
                div().child(format!("You are running the latest version, {CURRENT}."))
            }
            Looking::Failed(reason) => {
                parts::failure(format!("Looking for updates failed: {reason}"), cx)
            }
            Looking::Available(offered) => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(format!(
                    "Version {} is available. You are running {CURRENT}.",
                    offered.version()
                ))
                .children(
                    offered
                        .notes()
                        .map(|notes| div().text_sm().child(notes.to_owned())),
                )
                .child(parts::hint(
                    "Stashden closes and reopens on the new version. A running sync stops with it.",
                    cx,
                )),
        };
        let available = matches!(self.looking, Looking::Available(_));

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(body)
            .children(
                self.failure
                    .clone()
                    .map(|failure| parts::failure(failure, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("update-close")
                            .ghost()
                            .label(if available { "Later" } else { "Close" })
                            .disabled(self.installing)
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .when(available, |actions| {
                        actions.child(
                            Button::new("update-install")
                                .primary()
                                .label(if self.installing {
                                    "Installing"
                                } else {
                                    "Install and restart"
                                })
                                .disabled(self.installing)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.install(window, cx)),
                                ),
                        )
                    }),
            )
    }
}
