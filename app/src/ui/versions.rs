use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use stashden_core::node::Node;
use stashden_core::version::Version;

use super::parts::{self, Load};
use super::prompt;
use super::run;
use crate::describe;
use crate::desktop::{self, Desktop};
use crate::failure::Failure;

pub struct VersionRestored;

pub struct VersionsDialog {
    desktop: Arc<Desktop>,
    node: Node,
    path: String,
    can_restore: bool,
    versions: Load<Vec<Version>>,
    busy: bool,
    failure: Option<String>,
    announcement: Option<String>,
}

impl EventEmitter<VersionRestored> for VersionsDialog {}

pub fn open(
    desktop: Arc<Desktop>,
    node: Node,
    path: String,
    can_restore: bool,
    window: &mut Window,
    cx: &mut App,
) -> Entity<VersionsDialog> {
    let title = format!("Versions of {}", node.name);
    let dialog = cx.new(|cx| {
        let mut dialog = VersionsDialog {
            desktop,
            node,
            path,
            can_restore,
            versions: Load::Loading,
            busy: false,
            failure: None,
            announcement: None,
        };
        dialog.reload(window, cx);
        dialog
    });
    let content = dialog.clone();
    window.open_dialog(cx, move |shell, _, _| {
        shell
            .title(title.clone())
            .w(px(520.))
            .child(content.clone())
    });
    dialog
}

impl VersionsDialog {
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let path = self.path.clone();
        let work = async move { Ok::<_, Failure>(remote?.list_versions(&path).await?) };
        run(window, cx, work, |this, versions, _, cx| {
            this.versions = versions.into();
            cx.notify();
        });
    }

    fn download(&mut self, version: &Version, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let path = self.path.clone();
        let id = version.id;
        let work = async move {
            let destination = desktop::into_downloads(&path)?;
            remote?.download_version(&path, id, &destination).await?;
            Ok::<_, Failure>(destination)
        };
        self.busy = true;
        run(
            window,
            cx,
            work,
            |this, saved: Result<PathBuf, Failure>, _, cx| {
                this.busy = false;
                match saved {
                    Ok(saved) => this.announcement = Some(format!("Saved to {}", saved.display())),
                    Err(failure) => this.failure = Some(format!("The download failed: {failure}")),
                }
                cx.notify();
            },
        );
    }

    fn ask_restore(&mut self, version: &Version, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = cx.entity().downgrade();
        let id = version.id;
        prompt::confirm(
            format!("Restore {}?", self.node.name),
            format!(
                "It goes back to the version from {}. The current content is kept as a version, so this can be undone.",
                describe::moment(version.created_at)
            ),
            "Restore",
            window,
            cx,
            move |window, cx| {
                let _ = dialog.update(cx, |dialog, cx| dialog.restore(id, window, cx));
            },
        );
    }

    fn restore(&mut self, id: uuid::Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let path = self.path.clone();
        let work = async move { Ok::<_, Failure>(remote?.restore_version(&path, id).await?) };
        self.busy = true;
        run(window, cx, work, |this, restored, window, cx| {
            this.busy = false;
            match restored {
                Ok(_) => {
                    this.announcement = Some("Restored".to_owned());
                    cx.emit(VersionRestored);
                    this.reload(window, cx);
                }
                Err(failure) => this.failure = Some(format!("The restore failed: {failure}")),
            }
            cx.notify();
        });
    }
}

impl Render for VersionsDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.versions {
            Load::Loading => parts::state("Loading the versions", cx).into_any_element(),
            Load::Failed(error) => {
                parts::failure(format!("The versions did not load: {error}"), cx).into_any_element()
            }
            Load::Ready(versions) if versions.is_empty() => parts::state(
                "No earlier versions. This file has not been replaced yet.",
                cx,
            )
            .into_any_element(),
            Load::Ready(versions) => div()
                .flex()
                .flex_col()
                .children(versions.iter().enumerate().map(|(index, version)| {
                    let downloaded = version.clone();
                    let restored = version.clone();
                    parts::row(cx)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(describe::moment(version.created_at))
                                .child(parts::hint(describe::size(version.size), cx)),
                        )
                        .child(
                            Button::new(("version-download", index))
                                .ghost()
                                .compact()
                                .label("Download")
                                .disabled(self.busy)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.download(&downloaded, window, cx);
                                })),
                        )
                        .when(self.can_restore, |row| {
                            row.child(
                                Button::new(("version-restore", index))
                                    .ghost()
                                    .compact()
                                    .label("Restore")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.ask_restore(&restored, window, cx);
                                    })),
                            )
                        })
                }))
                .into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(parts::hint(
                "Each time the file is replaced, what it held before is kept here. The oldest go first when the list is full or the account runs out of room.",
                cx,
            ))
            .child(body)
            .children(self.failure.clone().map(|failure| parts::failure(failure, cx)))
            .children(self.announcement.clone().map(|done| parts::hint(done, cx)))
            .child(
                div().flex().flex_row().justify_end().child(
                    Button::new("versions-done")
                        .primary()
                        .label("Done")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                ),
            )
    }
}
