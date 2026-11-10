use std::sync::Arc;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use stashden_core::grant::{Access, Given, NewGrant};
use stashden_core::node::{Node, NodeKind};
use stashden_core::share::{Minted, NewShare};

use super::parts::{self, Load};
use super::prompt;
use super::run;
use crate::describe;
use crate::desktop::Desktop;
use crate::failure::Failure;

pub struct Published;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Link,
    Account,
}

pub struct ShareDialog {
    desktop: Arc<Desktop>,
    node: Node,
    path: String,
    mode: Mode,
    expiry: Entity<InputState>,
    password: Entity<InputState>,
    email: Entity<InputState>,
    access: Access,
    minted: Option<Minted>,
    copied: bool,
    busy: bool,
    failure: Option<String>,
    announcement: Option<String>,
    grants: Load<Vec<Given>>,
}

impl EventEmitter<Published> for ShareDialog {}

pub fn link_request(path: &str, expiry: &str, password: &str) -> Result<NewShare, String> {
    let expires_at = describe::end_of_day(expiry)?;
    let password = (!password.is_empty()).then(|| password.to_owned());
    if password
        .as_ref()
        .is_some_and(|password| password.chars().count() < 6)
    {
        return Err("a password must be at least six characters".to_owned());
    }
    Ok(NewShare {
        path: path.to_owned(),
        expires_at,
        password,
    })
}

impl ShareDialog {
    pub fn open(
        desktop: Arc<Desktop>,
        node: Node,
        path: String,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let title = format!("Share {}", node.name);
        let dialog = cx.new(|cx| Self::new(desktop, node, path, window, cx));
        let content = dialog.clone();
        window.open_dialog(cx, move |shell, _, _| {
            shell
                .title(title.clone())
                .w(px(520.))
                .child(content.clone())
        });
        dialog
    }

    fn new(
        desktop: Arc<Desktop>,
        node: Node,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut dialog = Self {
            desktop,
            node,
            path,
            mode: Mode::Link,
            expiry: cx.new(|cx| InputState::new(window, cx).placeholder("2026-12-31")),
            password: cx.new(|cx| InputState::new(window, cx).masked(true)),
            email: cx.new(|cx| InputState::new(window, cx).placeholder("someone@example.com")),
            access: Access::Read,
            minted: None,
            copied: false,
            busy: false,
            failure: None,
            announcement: None,
            grants: Load::Loading,
        };
        dialog.load_grants(window, cx);
        dialog
    }

    fn load_grants(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let node = self.node.id;
        let work = async move {
            let grants = remote?.list_grants().await?;
            Ok::<_, Failure>(
                grants
                    .into_iter()
                    .filter(|given| given.node_id == node)
                    .collect(),
            )
        };
        run(window, cx, work, |this, grants, _, cx| {
            this.grants = grants.into();
            cx.notify();
        });
    }

    fn url(&self) -> Option<String> {
        let minted = self.minted.as_ref()?;
        Some(describe::share_url(&self.desktop.server()?, &minted.token))
    }

    fn mint(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let request = link_request(
            &self.path,
            &self.expiry.read(cx).value(),
            &self.password.read(cx).value(),
        );
        let request = match request {
            Ok(request) => request,
            Err(reason) => {
                self.failure = Some(reason);
                cx.notify();
                return;
            }
        };
        self.busy = true;
        self.failure = None;
        cx.notify();
        let remote = self.desktop.remote();
        let work = async move { Ok::<_, Failure>(remote?.share(&request).await?) };
        run(window, cx, work, |this, minted, _, cx| {
            this.busy = false;
            match minted {
                Ok(minted) => {
                    this.minted = Some(minted);
                    cx.emit(Published);
                }
                Err(failure) => this.failure = Some(format!("The link was not created: {failure}")),
            }
            cx.notify();
        });
    }

    fn give(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let email = self.email.read(cx).value().trim().to_owned();
        if email.is_empty() {
            return;
        }
        let request = NewGrant {
            path: self.path.clone(),
            email: email.clone(),
            access: self.access,
        };
        self.busy = true;
        self.failure = None;
        cx.notify();
        let remote = self.desktop.remote();
        let work = async move { Ok::<_, Failure>(remote?.grant(&request).await?) };
        run(window, cx, work, move |this, given, window, cx| {
            this.busy = false;
            match given {
                Ok(_) => {
                    this.announcement = Some(format!("Shared with {email}"));
                    this.email
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    this.load_grants(window, cx);
                    cx.emit(Published);
                }
                Err(failure) => this.failure = Some(format!("Sharing failed: {failure}")),
            }
            cx.notify();
        });
    }

    fn ask_take(&mut self, given: &Given, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = cx.entity().downgrade();
        let id = given.id;
        let email = given.email.clone();
        prompt::confirm(
            format!("Stop sharing {} with {}?", self.node.name, given.email),
            "It disappears from their Shared with me on their next request. Your files are untouched."
                .to_owned(),
            "Stop sharing",
            window,
            cx,
            move |window, cx| {
                let email = email.clone();
                let _ = dialog.update(cx, |dialog, cx| {
                    let remote = dialog.desktop.remote();
                    let work = async move { Ok::<_, Failure>(remote?.withdraw_grant(id).await?) };
                    run(window, cx, work, move |dialog, outcome, window, cx| {
                        match outcome {
                            Ok(()) => {
                                dialog.announcement = Some(format!("Stopped sharing with {email}"));
                                cx.emit(Published);
                            }
                            Err(failure) => {
                                dialog.failure = Some(format!("Stopping failed: {failure}"));
                            }
                        }
                        dialog.load_grants(window, cx);
                    });
                });
            },
        );
    }

    fn field(label: &'static str, input: &Entity<InputState>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(label))
            .child(Input::new(input))
    }

    fn minted_view(&self, cx: &mut Context<Self>) -> Div {
        let url = self.url().unwrap_or_default();
        let protected = self
            .minted
            .as_ref()
            .is_some_and(|minted| minted.share.has_password);
        let copied_url = url.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(parts::hint(
                "This is the only time the link is shown. The server keeps a fingerprint of it and cannot show it again.",
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().text_sm().child(url))
                    .child(
                        Button::new("copy")
                            .primary()
                            .label(if self.copied { "Copied" } else { "Copy" })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copied_url.clone()));
                                this.copied = true;
                                cx.notify();
                            })),
                    ),
            )
            .when(protected, |view| {
                view.child(parts::hint(
                    "Whoever opens it will be asked for the password you set.",
                    cx,
                ))
            })
    }

    fn link_form(&self, cx: &mut Context<Self>) -> Div {
        let what = if self.node.kind == NodeKind::Directory {
            "browse and download this folder"
        } else {
            "download this file"
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(parts::hint(
                format!("Anyone holding the link can {what} without an account."),
                cx,
            ))
            .child(Self::field("Expires on", &self.expiry))
            .child(parts::hint(
                "Leave it empty for a link that does not expire. Write it as 2026-12-31.",
                cx,
            ))
            .child(Self::field("Password", &self.password))
            .child(parts::hint(
                "Optional, and at least six characters. It is not part of the link.",
                cx,
            ))
            .child(
                Button::new("mint")
                    .primary()
                    .label(if self.busy { "Creating" } else { "Create link" })
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, window, cx| this.mint(window, cx))),
            )
    }

    fn account_form(&self, cx: &mut Context<Self>) -> Div {
        let choice = |id: &'static str, label: &'static str, access: Access, current: Access| {
            let button = Button::new(id).label(label);
            let button = if access == current {
                button.primary()
            } else {
                button.ghost()
            };
            button.on_click(cx.listener(move |this, _, _, cx| {
                this.access = access;
                cx.notify();
            }))
        };
        let people = self.grants.ready().cloned().unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(parts::hint(
                "They find it under Shared with me. An address with no account yet receives it when one is made for it.",
                cx,
            ))
            .child(Self::field("Email address", &self.email))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(div().text_sm().child("They can"))
                    .child(choice("read", "View and download", Access::Read, self.access))
                    .child(choice("write", "Edit, add and delete", Access::Write, self.access)),
            )
            .when(self.access == Access::Write, |form| {
                form.child(parts::hint(
                    "What they add or change is stored in your files and counts against your storage.",
                    cx,
                ))
            })
            .child(
                Button::new("give")
                    .primary()
                    .label(if self.busy { "Sharing" } else { "Share" })
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, window, cx| this.give(window, cx))),
            )
            .when(!people.is_empty(), |form| {
                form.child(parts::heading("Shared with")).children(
                    people.into_iter().enumerate().map(|(index, given)| {
                        let taken = given.clone();
                        parts::row(cx)
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .child(given.email.clone())
                                    .child(parts::hint(describe::access(given.access), cx)),
                            )
                            .child(
                                Button::new(("take", index))
                                    .danger()
                                    .compact()
                                    .label("Remove")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.ask_take(&taken, window, cx);
                                    })),
                            )
                    }),
                )
            })
    }
}

impl Render for ShareDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.minted.is_some() {
            self.minted_view(cx)
        } else {
            let form = match self.mode {
                Mode::Link => self.link_form(cx),
                Mode::Account => self.account_form(cx),
            };
            let mode = |id: &'static str, label: &'static str, mode: Mode, current: Mode| {
                let button = Button::new(id).label(label);
                let button = if mode == current {
                    button.primary()
                } else {
                    button.ghost()
                };
                button.on_click(cx.listener(move |this, _, _, cx| {
                    this.mode = mode;
                    this.failure = None;
                    cx.notify();
                }))
            };
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .child(mode("mode-link", "With a link", Mode::Link, self.mode))
                        .child(mode(
                            "mode-account",
                            "With an account",
                            Mode::Account,
                            self.mode,
                        )),
                )
                .child(form)
        };
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
            .children(self.announcement.clone().map(|done| parts::hint(done, cx)))
            .child(
                div().flex().flex_row().justify_end().child(
                    Button::new("share-done")
                        .ghost()
                        .label("Done")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::link_request;

    #[test]
    fn an_empty_form_asks_for_a_link_that_never_expires_and_has_no_password() {
        let request = link_request("docs/a.txt", "", "").unwrap();
        assert_eq!(request.path, "docs/a.txt");
        assert_eq!(request.expires_at, None);
        assert_eq!(request.password, None);
    }

    #[test]
    fn a_short_password_or_a_bad_date_is_refused_before_asking_the_server() {
        assert!(link_request("a", "", "12345").unwrap_err().contains("six"));
        assert!(link_request("a", "next week", "").is_err());
        let request = link_request("a", "2031-01-01", "secret").unwrap();
        assert!(request.expires_at.is_some());
        assert_eq!(request.password.as_deref(), Some("secret"));
    }
}
