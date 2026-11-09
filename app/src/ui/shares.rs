use std::sync::Arc;

use chrono::Utc;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;
use stashden_core::grant::Given;
use stashden_core::share::Share;

use super::parts::{self, Load};
use super::prompt;
use super::run;
use super::search::panel;
use crate::describe;
use crate::desktop::Desktop;
use crate::failure::Failure;

pub struct SharesPanel {
    desktop: Arc<Desktop>,
    links: Load<Vec<Share>>,
    grants: Load<Vec<Given>>,
    failure: Option<String>,
}

pub fn given_summary(given: &Given) -> String {
    let mut summary = format!("With {}, {}", given.email, describe::access(given.access));
    if given.in_trash {
        summary.push_str(", in the trash");
    }
    summary
}

impl SharesPanel {
    pub fn new(desktop: Arc<Desktop>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            desktop,
            links: Load::Loading,
            grants: Load::Loading,
            failure: None,
        };
        panel.reload(window, cx);
        panel
    }

    pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let links = async move { Ok::<_, Failure>(remote?.list_shares().await?) };
        run(window, cx, links, |this, links, _, cx| {
            this.links = links.into();
            cx.notify();
        });
        let remote = self.desktop.remote();
        let grants = async move { Ok::<_, Failure>(remote?.list_grants().await?) };
        run(window, cx, grants, |this, grants, _, cx| {
            this.grants = grants.into();
            cx.notify();
        });
    }

    fn finish(
        &mut self,
        what: &str,
        outcome: Result<(), Failure>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.failure = outcome
            .err()
            .map(|failure| format!("{what} failed: {failure}"));
        self.reload(window, cx);
    }

    fn ask_revoke(link: &Share, window: &mut Window, cx: &mut Context<Self>) {
        let panel = cx.entity().downgrade();
        let id = link.id;
        let name = link.name.clone();
        prompt::confirm(
            format!("Revoke the link to {name}?"),
            "Anyone holding it stops being able to open it. The file itself is untouched."
                .to_owned(),
            "Revoke",
            window,
            cx,
            move |window, cx| {
                let name = name.clone();
                let _ = panel.update(cx, |panel, cx| {
                    let remote = panel.desktop.remote();
                    let work = async move { Ok::<_, Failure>(remote?.revoke_share(id).await?) };
                    run(window, cx, work, move |panel, outcome, window, cx| {
                        panel.finish(&format!("revoking the link to {name}"), outcome, window, cx);
                    });
                });
            },
        );
    }

    fn ask_withdraw(given: &Given, window: &mut Window, cx: &mut Context<Self>) {
        let panel = cx.entity().downgrade();
        let id = given.id;
        let name = given.name.clone();
        prompt::confirm(
            format!("Stop sharing {} with {}?", given.name, given.email),
            "It disappears from their Shared with me on their next request. Your files are untouched."
                .to_owned(),
            "Stop sharing",
            window,
            cx,
            move |window, cx| {
                let name = name.clone();
                let _ = panel.update(cx, |panel, cx| {
                    let remote = panel.desktop.remote();
                    let work = async move { Ok::<_, Failure>(remote?.withdraw_grant(id).await?) };
                    run(window, cx, work, move |panel, outcome, window, cx| {
                        panel.finish(&format!("stopping sharing {name}"), outcome, window, cx);
                    });
                });
            },
        );
    }
}

impl Render for SharesPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Utc::now();
        let links = match &self.links {
            Load::Loading => parts::state("Loading your links", cx).into_any_element(),
            Load::Failed(error) => {
                parts::failure(format!("Your links did not load: {error}"), cx).into_any_element()
            }
            Load::Ready(links) if links.is_empty() => {
                parts::state("You have not published any links.", cx).into_any_element()
            }
            Load::Ready(links) => div()
                .flex()
                .flex_col()
                .children(links.iter().enumerate().map(|(index, link)| {
                    let doomed = link.clone();
                    parts::row(cx)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(link.name.clone())
                                .child(parts::hint(describe::link(link, now), cx)),
                        )
                        .child(
                            Button::new(("revoke", index))
                                .danger()
                                .compact()
                                .label("Revoke")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    Self::ask_revoke(&doomed, window, cx);
                                })),
                        )
                }))
                .into_any_element(),
        };
        let grants = match &self.grants {
            Load::Loading => parts::state("Loading what you share", cx).into_any_element(),
            Load::Failed(error) => {
                parts::failure(format!("What you share did not load: {error}"), cx)
                    .into_any_element()
            }
            Load::Ready(grants) if grants.is_empty() => {
                parts::state("You do not share anything with another account.", cx)
                    .into_any_element()
            }
            Load::Ready(grants) => div()
                .flex()
                .flex_col()
                .children(grants.iter().enumerate().map(|(index, given)| {
                    let withdrawn = given.clone();
                    parts::row(cx)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(given.name.clone())
                                .child(parts::hint(given_summary(given), cx)),
                        )
                        .child(
                            Button::new(("withdraw", index))
                                .danger()
                                .compact()
                                .label("Stop sharing")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    Self::ask_withdraw(&withdrawn, window, cx);
                                })),
                        )
                }))
                .into_any_element(),
        };
        panel("Links you have published", cx)
            .children(
                self.failure
                    .clone()
                    .map(|failure| parts::failure(failure, cx)),
            )
            .child(links)
            .child(parts::heading("Shared with other accounts"))
            .child(grants)
    }
}
