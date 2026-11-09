use std::sync::Arc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use stashden_core::user::User;

use super::files::FileBrowser;
use super::parts;
use super::prompt::{self, Ask};
use super::run;
use super::sync_view::SyncView;
use super::update;
use crate::describe;
use crate::desktop::Desktop;

pub struct SignedOut {
    pub reason: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Files,
    Sync,
}

pub struct Shell {
    desktop: Arc<Desktop>,
    account: Option<User>,
    page: Page,
    files: Entity<FileBrowser>,
    sync: Entity<SyncView>,
    notice: Option<String>,
    error: Option<String>,
    menu_open: bool,
}

impl EventEmitter<SignedOut> for Shell {}

impl Shell {
    pub fn new(
        desktop: Arc<Desktop>,
        notice: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let files = cx.new(|cx| FileBrowser::new(Arc::clone(&desktop), window, cx));
        let sync = cx.new(|cx| SyncView::new(Arc::clone(&desktop), cx));
        let shell = Self {
            desktop,
            account: None,
            page: Page::Files,
            files,
            sync,
            notice,
            error: None,
            menu_open: false,
        };
        shell.load_account(window, cx);
        shell
    }

    fn load_account(&self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let work = async move { remote?.me().await.map_err(crate::failure::Failure::from) };
        run(
            window,
            cx,
            work,
            |this, account, window, cx| match account {
                Ok(account) => {
                    this.files.update(cx, |files, cx| {
                        files.set_can_write(account.role.may_write(), cx);
                    });
                    this.account = Some(account);
                    cx.notify();
                }
                Err(failure) if failure.ends_the_session() => this.sign_out(window, cx),
                Err(failure) => {
                    this.error = Some(format!("Your account did not load: {failure}"));
                    cx.notify();
                }
            },
        );
    }

    fn sign_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_open = false;
        let desktop = Arc::clone(&self.desktop);
        run(
            window,
            cx,
            async move { desktop.sign_out().await },
            |_, outcome, _, cx| {
                cx.emit(SignedOut {
                    reason: outcome
                        .err()
                        .map(|failure| format!("Signed out here, but {failure}")),
                });
            },
        );
    }

    fn rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_open = false;
        let current = self
            .account
            .as_ref()
            .map(|account| account.display_name.clone())
            .unwrap_or_default();
        let shell = cx.entity().downgrade();
        prompt::ask(
            Ask {
                heading: "Display name".to_owned(),
                label: "Name",
                hint: None,
                value: current,
                action: "Save",
            },
            window,
            cx,
            move |name, window, cx| {
                let _ = shell.update(cx, |shell, cx| shell.save_name(name, window, cx));
            },
        );
    }

    fn save_name(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let work = async move {
            remote?
                .update_display_name(&name)
                .await
                .map_err(crate::failure::Failure::from)
        };
        run(window, cx, work, |this, renamed, _, cx| {
            match renamed {
                Ok(account) => {
                    this.account = Some(account);
                    this.error = None;
                    this.notice = Some("Your display name has been changed.".to_owned());
                }
                Err(failure) => {
                    this.error = Some(format!("Your display name was not changed: {failure}"));
                }
            }
            cx.notify();
        });
    }

    fn go(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        self.menu_open = false;
        cx.notify();
    }

    fn account_menu(&self, account: &User, cx: &mut Context<Self>) -> Div {
        let item = |id: &'static str, label: &'static str| Button::new(id).ghost().label(label);
        div()
            .relative()
            .child(
                Button::new("account")
                    .ghost()
                    .label(account.display_name.clone())
                    .dropdown_caret(true)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = !this.menu_open;
                        cx.notify();
                    })),
            )
            .when(self.menu_open, |anchor| {
                anchor.child(deferred(
                    anchored().anchor(Anchor::TopRight).snap_to_window().child(
                        div()
                            .id("account-menu")
                            .mt_10()
                            .w(px(240.))
                            .p_2()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().popover)
                            .shadow_md()
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.menu_open = false;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .px_2()
                                    .pb_2()
                                    .child(account.display_name.clone())
                                    .child(parts::hint(account.email.clone(), cx))
                                    .child(parts::hint(describe::role(account.role), cx)),
                            )
                            .child(
                                item("menu-sync", "Sync a folder").on_click(
                                    cx.listener(|this, _, _, cx| this.go(Page::Sync, cx)),
                                ),
                            )
                            .child(item("menu-name", "Change display name").on_click(
                                cx.listener(|this, _, window, cx| this.rename(window, cx)),
                            ))
                            .child(
                                item("menu-update", "Check for updates").on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.menu_open = false;
                                        update::open(window, cx);
                                    },
                                )),
                            )
                            .child(item("menu-out", "Sign out").on_click(
                                cx.listener(|this, _, window, cx| this.sign_out(window, cx)),
                            )),
                    ),
                ))
            })
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = |id: &'static str, label: &'static str, page: Page, current: Page| {
            let button = Button::new(id).label(label);
            if page == current {
                button.primary()
            } else {
                button.ghost()
            }
        };
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(div().font_weight(FontWeight::BOLD).mr_4().child("Stashden"))
                    .child(
                        tab("page-files", "Files", Page::Files, self.page)
                            .on_click(cx.listener(|this, _, _, cx| this.go(Page::Files, cx))),
                    )
                    .child(
                        tab("page-sync", "Sync", Page::Sync, self.page)
                            .on_click(cx.listener(|this, _, _, cx| this.go(Page::Sync, cx))),
                    ),
            )
            .children(
                self.account
                    .clone()
                    .map(|account| self.account_menu(&account, cx)),
            );

        let page = match self.page {
            Page::Files => self.files.clone().into_any_element(),
            Page::Sync => self.sync.clone().into_any_element(),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px_4()
                    .pt_2()
                    .children(self.notice.clone().map(|notice| parts::notice(notice, cx)))
                    .children(self.error.clone().map(|error| parts::failure(error, cx))),
            )
            .child(div().flex_1().min_h_0().child(page))
    }
}
