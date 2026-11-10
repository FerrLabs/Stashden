use std::sync::Arc;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use stashden_core::node::{NodeKind, Trashed};

use super::parts::{self, Load};
use super::prompt;
use super::run;
use super::search::panel;
use crate::describe;
use crate::desktop::Desktop;
use crate::failure::Failure;

pub struct Restored;

pub struct TrashPanel {
    desktop: Arc<Desktop>,
    can_change: bool,
    trashed: Load<Vec<Trashed>>,
    busy: bool,
    failure: Option<String>,
    announcement: Option<String>,
}

impl EventEmitter<Restored> for TrashPanel {}

pub fn summary(node: &Trashed) -> String {
    let mut parts = vec![match node.node.kind {
        NodeKind::Directory => "Folder".to_owned(),
        NodeKind::File => describe::size(node.node.size),
    }];
    if let Some(at) = node.node.deleted_at {
        parts.push(format!("deleted {}", describe::day(at)));
    }
    if let Some(at) = node.expires_at {
        parts.push(format!("gone for good {}", describe::day(at)));
    }
    parts.join(", ")
}

impl TrashPanel {
    pub fn new(
        desktop: Arc<Desktop>,
        can_change: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut panel = Self {
            desktop,
            can_change,
            trashed: Load::Loading,
            busy: false,
            failure: None,
            announcement: None,
        };
        panel.reload(window, cx);
        panel
    }

    pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let work = async move { Ok::<_, Failure>(remote?.trash().await?) };
        run(window, cx, work, |this, trashed, _, cx| {
            this.trashed = trashed.into();
            cx.notify();
        });
    }

    fn act<T: Send + 'static>(
        &mut self,
        what: String,
        work: impl Future<Output = Result<T, Failure>> + Send + 'static,
        done: String,
        restored: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.busy = true;
        self.failure = None;
        cx.notify();
        run(window, cx, work, move |this, outcome, window, cx| {
            this.busy = false;
            match outcome {
                Ok(_) => {
                    this.announcement = Some(done);
                    if restored {
                        cx.emit(Restored);
                    }
                }
                Err(failure) => this.failure = Some(format!("{what} failed: {failure}")),
            }
            this.reload(window, cx);
        });
    }

    fn restore(&mut self, node: &Trashed, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let id = node.node.id;
        let name = node.node.name.clone();
        let work = async move { Ok::<_, Failure>(remote?.restore(id).await?) };
        self.act(
            format!("restoring {name}"),
            work,
            format!("Restored {name}"),
            true,
            window,
            cx,
        );
    }

    fn purge(&mut self, node: &Trashed, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let id = node.node.id;
        let name = node.node.name.clone();
        let work = async move { Ok::<_, Failure>(remote?.purge(id).await?) };
        self.act(
            format!("deleting {name}"),
            work,
            format!("Deleted {name} for good"),
            false,
            window,
            cx,
        );
    }

    fn empty(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let work = async move { Ok::<_, Failure>(remote?.empty_trash().await?) };
        self.act(
            "emptying the trash".to_owned(),
            work,
            "The trash is empty".to_owned(),
            false,
            window,
            cx,
        );
    }

    fn ask_purge(node: &Trashed, window: &mut Window, cx: &mut Context<Self>) {
        let panel = cx.entity().downgrade();
        let doomed = node.clone();
        let body = if node.node.kind == NodeKind::Directory {
            "The folder and everything in it are deleted, with their versions. This cannot be undone."
        } else {
            "The file is deleted, with its versions. This cannot be undone."
        };
        prompt::confirm(
            format!("Delete {} for good?", node.node.name),
            body.to_owned(),
            "Delete forever",
            window,
            cx,
            move |window, cx| {
                let _ = panel.update(cx, |panel, cx| panel.purge(&doomed, window, cx));
            },
        );
    }

    fn ask_empty(window: &mut Window, cx: &mut Context<Self>) {
        let panel = cx.entity().downgrade();
        prompt::confirm(
            "Empty the trash?".to_owned(),
            "Everything in it is deleted for good, with its versions. This cannot be undone."
                .to_owned(),
            "Empty the trash",
            window,
            cx,
            move |window, cx| {
                let _ = panel.update(cx, |panel, cx| panel.empty(window, cx));
            },
        );
    }
}

impl Render for TrashPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_items = self
            .trashed
            .ready()
            .is_some_and(|trashed| !trashed.is_empty());
        let body = match &self.trashed {
            Load::Loading => parts::state("Loading the trash", cx).into_any_element(),
            Load::Failed(error) => {
                parts::failure(format!("The trash did not load: {error}"), cx).into_any_element()
            }
            Load::Ready(trashed) if trashed.is_empty() => {
                parts::state("The trash is empty.", cx).into_any_element()
            }
            Load::Ready(trashed) => div()
                .flex()
                .flex_col()
                .children(trashed.iter().enumerate().map(|(index, node)| {
                    let restored = node.clone();
                    let doomed = node.clone();
                    parts::row(cx)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(node.node.name.clone())
                                .child(parts::hint(summary(node), cx)),
                        )
                        .when(self.can_change, |row| {
                            row.child(
                                Button::new(("restore", index))
                                    .ghost()
                                    .compact()
                                    .label("Restore")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.restore(&restored, window, cx);
                                    })),
                            )
                            .child(
                                Button::new(("purge", index))
                                    .danger()
                                    .compact()
                                    .label("Delete forever")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |_, _, window, cx| {
                                        Self::ask_purge(&doomed, window, cx);
                                    })),
                            )
                        })
                }))
                .into_any_element(),
        };
        panel("Trash", cx)
            .when(self.can_change && has_items, |panel| {
                panel.child(
                    div().child(
                        Button::new("empty-trash")
                            .danger()
                            .label("Empty the trash")
                            .disabled(self.busy)
                            .on_click(cx.listener(|_, _, window, cx| Self::ask_empty(window, cx))),
                    ),
                )
            })
            .children(
                self.failure
                    .clone()
                    .map(|failure| parts::failure(failure, cx)),
            )
            .children(self.announcement.clone().map(|done| parts::hint(done, cx)))
            .child(body)
    }
}
