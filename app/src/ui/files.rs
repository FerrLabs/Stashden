use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use stashden_core::grant::Received;
use stashden_core::node::{Node, NodeKind};

use super::parts::{self, Load};
use super::preview;
use super::prompt::{self, Ask};
use super::run;
use super::search::{Chosen, SearchPanel};
use super::share::{Published, ShareDialog};
use super::shares::SharesPanel;
use super::trash::{Restored, TrashPanel};
use super::versions::{self, VersionRestored};
use crate::describe;
use crate::desktop::{self, Desktop};
use crate::failure::Failure;
use crate::location::{Folder, Where, sorted};

pub struct FileBrowser {
    desktop: Arc<Desktop>,
    folder: Folder,
    listing: Load<Vec<Node>>,
    received: Option<Load<Vec<Received>>>,
    can_write: bool,
    failure: Option<String>,
    announcement: Option<String>,
    uploading: Option<String>,
    loads: u64,
    search: Option<Entity<SearchPanel>>,
    shares: Option<Entity<SharesPanel>>,
    trash: Option<Entity<TrashPanel>>,
}

impl FileBrowser {
    pub fn new(desktop: Arc<Desktop>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut browser = Self {
            desktop,
            folder: Folder::root(),
            listing: Load::Loading,
            received: None,
            can_write: false,
            failure: None,
            announcement: None,
            uploading: None,
            loads: 0,
            search: None,
            shares: None,
            trash: None,
        };
        browser.reload(window, cx);
        browser
    }

    pub fn set_can_write(&mut self, can_write: bool, cx: &mut Context<Self>) {
        self.can_write = can_write;
        cx.notify();
    }

    fn mount(&self) -> Option<&Received> {
        self.received
            .as_ref()
            .and_then(Load::ready)
            .and_then(|received| self.folder.mount(received))
    }

    fn can_change(&self) -> bool {
        let allowed = match self.folder.place() {
            Where::Own => true,
            Where::Shelf => false,
            Where::Shared { .. } => self.mount().is_some_and(|mount| mount.access.may_write()),
        };
        self.can_write && allowed
    }

    fn can_share(&self) -> bool {
        self.can_write && self.folder.place() == Where::Own
    }

    fn navigate(&mut self, folder: Folder, window: &mut Window, cx: &mut Context<Self>) {
        self.folder = folder;
        self.failure = None;
        self.announcement = None;
        self.reload(window, cx);
    }

    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.loads += 1;
        let load = self.loads;
        self.listing = Load::Loading;
        let remote = self.desktop.remote();
        let path = self.folder.remote();
        let work = async move {
            let remote = remote?;
            remote
                .list(&path)
                .await
                .map_err(|error| Failure::from(error).about(&path))
        };
        run(window, cx, work, move |this, listed, _, cx| {
            if this.loads == load {
                this.listing = listed.map(sorted).into();
                cx.notify();
            }
        });

        if self.folder.place() == Where::Own {
            self.received = None;
        } else {
            self.load_received(window, cx);
        }
        cx.notify();
    }

    fn load_received(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.received = Some(Load::Loading);
        let remote = self.desktop.remote();
        let work = async move { Ok::<_, Failure>(remote?.received().await?) };
        run(window, cx, work, |this, received, _, cx| {
            this.received = Some(received.into());
            cx.notify();
        });
    }

    fn attempt<T: Send + 'static>(
        what: String,
        work: impl Future<Output = Result<T, Failure>> + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, T, &mut Window, &mut Context<Self>) + 'static,
    ) {
        run(window, cx, work, move |this, outcome, window, cx| {
            match outcome {
                Ok(value) => {
                    this.failure = None;
                    done(this, value, window, cx);
                }
                Err(failure) => {
                    this.announcement = None;
                    this.failure = Some(format!("{what} failed: {failure}"));
                }
            }
            cx.notify();
        });
    }

    fn open(&mut self, node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        if node.kind == NodeKind::Directory {
            let folder = self.folder.enter(&node.name);
            self.navigate(folder, window, cx);
        } else {
            let browser = cx.entity().downgrade();
            let opened = node.clone();
            preview::open(
                &self.desktop,
                node,
                self.folder.child(&node.name),
                window,
                cx,
                move |window, cx| {
                    let _ = browser.update(cx, |browser, cx| browser.download(&opened, window, cx));
                },
            );
        }
    }

    fn download(&mut self, node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let path = self.folder.child(&node.name);
        let name = node.name.clone();
        let work = async move {
            let destination = desktop::into_downloads(&path)?;
            remote?.download(&path, &destination).await?;
            Ok(destination)
        };
        Self::attempt(
            format!("downloading {name}"),
            work,
            window,
            cx,
            move |this, saved: PathBuf, _, _| {
                this.announcement = Some(format!("Saved {name} to {}", saved.display()));
            },
        );
    }

    fn download_folder(&mut self, node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let path = self.folder.child(&node.name);
        let name = node.name.clone();
        let work = async move {
            let destination = desktop::into_downloads(&format!("{path}.zip"))?;
            remote?.download_archive(&path, &destination).await?;
            Ok(destination)
        };
        Self::attempt(
            format!("downloading {name}"),
            work,
            window,
            cx,
            move |this, saved: PathBuf, _, _| {
                this.announcement = Some(format!("Saved {name}.zip to {}", saved.display()));
            },
        );
    }

    fn ask_rename(node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        let browser = cx.entity().downgrade();
        let node = node.clone();
        prompt::ask(
            Ask {
                heading: format!("Rename {}", node.name),
                label: "New name",
                hint: Some("Type a path with / to move it into a folder here."),
                value: node.name.clone(),
                action: "Rename",
            },
            window,
            cx,
            move |destination, window, cx| {
                let _ = browser.update(cx, |browser, cx| {
                    browser.rename(&node, &destination, window, cx);
                });
            },
        );
    }

    fn rename(
        &mut self,
        node: &Node,
        destination: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let remote = self.desktop.remote();
        let from = self.folder.child(&node.name);
        let to = self.folder.child(destination);
        let name = node.name.clone();
        let moved = destination.contains('/');
        let target = to.clone();
        let work = async move { Ok::<_, Failure>(remote?.rename(&from, &to).await?) };
        Self::attempt(
            format!("renaming {name}"),
            work,
            window,
            cx,
            move |this, _, window, cx| {
                this.announcement = Some(if moved {
                    format!("Moved {name} to {target}")
                } else {
                    format!("Renamed {name}")
                });
                this.reload(window, cx);
            },
        );
    }

    fn ask_new_folder(window: &mut Window, cx: &mut Context<Self>) {
        let browser = cx.entity().downgrade();
        prompt::ask(
            Ask {
                heading: "New folder".to_owned(),
                label: "Name",
                hint: Some("Type a path with / to make folders inside folders."),
                value: String::new(),
                action: "Create",
            },
            window,
            cx,
            move |name, window, cx| {
                let _ = browser.update(cx, |browser, cx| browser.create_folder(name, window, cx));
            },
        );
    }

    fn create_folder(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let path = self.folder.child(&name);
        let work = async move { Ok::<_, Failure>(remote?.create_folder(&path).await?) };
        Self::attempt(
            format!("creating {name}"),
            work,
            window,
            cx,
            move |this, _, window, cx| {
                this.announcement = Some(format!("Created {name}"));
                this.reload(window, cx);
            },
        );
    }

    fn ask_remove(&mut self, node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        let directory = node.kind == NodeKind::Directory;
        let own = self.folder.place() == Where::Own;
        let body = format!(
            "{}{}",
            if directory {
                "The folder and everything in it move to the trash"
            } else {
                "The file moves to the trash"
            },
            match (own, directory) {
                (true, true) => ", where you can restore them.",
                (true, false) => ", where you can restore it.",
                (false, _) => " of the account that shares it with you.",
            }
        );
        let browser = cx.entity().downgrade();
        let node = node.clone();
        prompt::confirm(
            format!("Delete {}?", node.name),
            body,
            "Delete",
            window,
            cx,
            move |window, cx| {
                let _ = browser.update(cx, |browser, cx| browser.remove(&node, window, cx));
            },
        );
    }

    fn remove(&mut self, node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let path = self.folder.child(&node.name);
        let name = node.name.clone();
        let work = async move { Ok::<_, Failure>(remote?.delete(&path).await?) };
        Self::attempt(
            format!("deleting {name}"),
            work,
            window,
            cx,
            move |this, (), window, cx| {
                this.announcement = Some(format!("Moved {name} to the trash"));
                if let Some(trash) = &this.trash {
                    trash.update(cx, |trash, cx| trash.reload(window, cx));
                }
                this.reload(window, cx);
            },
        );
    }

    fn ask_leave(mount: &Received, window: &mut Window, cx: &mut Context<Self>) {
        let browser = cx.entity().downgrade();
        let mount = mount.clone();
        prompt::confirm(
            format!("Leave {}?", mount.name),
            format!(
                "It disappears from Shared with me. {} keeps it, and can share it with you again.",
                describe::owner(&mount)
            ),
            "Leave",
            window,
            cx,
            move |window, cx| {
                let _ = browser.update(cx, |browser, cx| browser.leave(&mount, window, cx));
            },
        );
    }

    fn leave(&mut self, mount: &Received, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.desktop.remote();
        let id = mount.id;
        let name = mount.name.clone();
        let work = async move { Ok::<_, Failure>(remote?.withdraw_grant(id).await?) };
        Self::attempt(
            format!("leaving {name}"),
            work,
            window,
            cx,
            move |this, (), window, cx| {
                this.announcement = Some(format!("Left {name}"));
                this.reload(window, cx);
            },
        );
    }

    fn pick_uploads(window: &mut Window, cx: &mut Context<Self>) {
        let picking = async {
            rfd::AsyncFileDialog::new()
                .pick_files()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|file| file.path().to_path_buf())
                .collect::<Vec<_>>()
        };
        run(window, cx, picking, |this, sources, window, cx| {
            this.upload(sources, window, cx);
        });
    }

    fn upload(&mut self, sources: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        if sources.is_empty() || !self.can_change() {
            return;
        }
        let desktop = Arc::clone(&self.desktop);
        let folder = self.folder.clone();
        let browser = cx.entity().downgrade();
        self.failure = None;
        self.uploading = Some(desktop::file_name(&sources[0]));
        cx.notify();
        cx.spawn_in(window, async move |_, cx| {
            let mut sent = 0;
            let mut failures = Vec::new();
            for source in sources {
                let name = desktop::file_name(&source);
                let _ = browser.update(cx, |browser, cx| {
                    browser.uploading = Some(name.clone());
                    cx.notify();
                });
                let desktop = Arc::clone(&desktop);
                let path = folder.child(&name);
                let outcome =
                    crate::runtime::spawn(async move { desktop.upload(path, source).await }).await;
                match outcome {
                    Ok(_) => sent += 1,
                    Err(failure) => failures.push(format!("uploading {name} failed: {failure}")),
                }
            }
            let _ = browser.update_in(cx, |browser, window, cx| {
                browser.uploading = None;
                browser.failure = (!failures.is_empty()).then(|| failures.join("\n"));
                if sent > 0 {
                    browser.announcement = Some(if sent == 1 {
                        "Uploaded 1 file".to_owned()
                    } else {
                        format!("Uploaded {sent} files")
                    });
                    browser.reload(window, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn share(&mut self, node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = ShareDialog::open(
            Arc::clone(&self.desktop),
            node.clone(),
            self.folder.child(&node.name),
            window,
            cx,
        );
        cx.subscribe_in(&dialog, window, |this, _, _: &Published, window, cx| {
            if let Some(shares) = &this.shares {
                shares.update(cx, |shares, cx| shares.reload(window, cx));
            }
        })
        .detach();
    }

    fn versions(&mut self, node: &Node, window: &mut Window, cx: &mut Context<Self>) {
        let dialog = versions::open(
            Arc::clone(&self.desktop),
            node.clone(),
            self.folder.child(&node.name),
            self.can_change(),
            window,
            cx,
        );
        cx.subscribe_in(
            &dialog,
            window,
            |this, _, _: &VersionRestored, window, cx| {
                this.reload(window, cx);
            },
        )
        .detach();
    }

    fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.take().is_none() {
            let panel = cx.new(|cx| SearchPanel::new(Arc::clone(&self.desktop), window, cx));
            cx.subscribe_in(&panel, window, |this, _, chosen: &Chosen, window, cx| {
                this.search = None;
                this.navigate(chosen.0.clone(), window, cx);
            })
            .detach();
            self.search = Some(panel);
        }
        cx.notify();
    }

    fn toggle_shares(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shares.take().is_none() {
            self.shares =
                Some(cx.new(|cx| SharesPanel::new(Arc::clone(&self.desktop), window, cx)));
        }
        cx.notify();
    }

    fn toggle_trash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.trash.take().is_none() {
            let can_change = self.can_write;
            let panel =
                cx.new(|cx| TrashPanel::new(Arc::clone(&self.desktop), can_change, window, cx));
            cx.subscribe_in(&panel, window, |this, _, _: &Restored, window, cx| {
                this.reload(window, cx);
            })
            .detach();
            self.trash = Some(panel);
        }
        cx.notify();
    }

    fn breadcrumb(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let crumb = |id: usize, name: String, folder: Folder| {
            Button::new(("crumb", id))
                .ghost()
                .compact()
                .label(name)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.navigate(folder.clone(), window, cx);
                }))
        };
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .child(crumb(0, "Files".to_owned(), Folder::root()))
            .children(
                self.folder
                    .crumbs()
                    .into_iter()
                    .enumerate()
                    .flat_map(|(index, part)| {
                        [
                            div().child("/").into_any_element(),
                            crumb(index + 1, part.name, part.folder).into_any_element(),
                        ]
                    }),
            )
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let toggle = |id: &'static str, label: &'static str, open: bool| {
            let button = Button::new(id).label(label);
            if open {
                button.primary()
            } else {
                button.ghost()
            }
        };
        let can_change = self.can_change();
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_2()
            .child(self.breadcrumb(cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .child(
                        Button::new("refresh")
                            .ghost()
                            .label("Refresh")
                            .disabled(matches!(self.listing, Load::Loading))
                            .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
                    )
                    .child(toggle("search", "Search", self.search.is_some()).on_click(
                        cx.listener(|this, _, window, cx| this.toggle_search(window, cx)),
                    ))
                    .child(
                        toggle("sharing", "Sharing", self.shares.is_some()).on_click(
                            cx.listener(|this, _, window, cx| this.toggle_shares(window, cx)),
                        ),
                    )
                    .child(
                        toggle("trash", "Trash", self.trash.is_some()).on_click(
                            cx.listener(|this, _, window, cx| this.toggle_trash(window, cx)),
                        ),
                    )
                    .when(can_change, |bar| {
                        bar.child(
                            Button::new("new-folder")
                                .ghost()
                                .label("New folder")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    Self::ask_new_folder(window, cx);
                                })),
                        )
                        .child(
                            Button::new("upload")
                                .primary()
                                .label(if self.uploading.is_some() {
                                    "Uploading"
                                } else {
                                    "Upload"
                                })
                                .disabled(self.uploading.is_some())
                                .on_click(
                                    cx.listener(|_, _, window, cx| Self::pick_uploads(window, cx)),
                                ),
                        )
                    }),
            )
    }

    fn banners(&self, cx: &mut Context<Self>) -> Div {
        let mut banners = div().flex().flex_col().gap_2();
        if matches!(self.received, Some(Load::Failed(_))) {
            banners = banners.child(
                parts::failure(
                    "Who shared this with you, and how, did not load, so nothing here can be changed for now.",
                    cx,
                )
                .child(
                    Button::new("received-retry")
                        .ghost()
                        .label("Try again")
                        .on_click(cx.listener(|this, _, window, cx| this.load_received(window, cx))),
                ),
            );
        }
        if let Some(mount) = self.mount() {
            let how = if mount.access.may_write() {
                "What you add or change here is stored in their files."
            } else {
                "You can view and download what is in it, and not change it."
            };
            banners = banners.child(parts::notice(
                format!("{} shares this with you. {how}", describe::owner(mount)),
                cx,
            ));
        }
        if let Some(failure) = &self.failure {
            banners = banners.child(parts::failure(failure.clone(), cx));
        }
        if let Some(name) = &self.uploading {
            banners = banners.child(parts::notice(format!("Uploading {name}"), cx));
        }
        if let Some(announcement) = &self.announcement {
            banners = banners.child(parts::hint(announcement.clone(), cx));
        }
        banners
    }

    fn shelf_mount(&self, node: &Node) -> Option<Received> {
        if self.folder.place() != Where::Shelf {
            return None;
        }
        self.received
            .as_ref()
            .and_then(Load::ready)
            .and_then(|received| received.iter().find(|grant| grant.name == node.name))
            .cloned()
    }

    fn actions(
        &self,
        index: usize,
        node: &Node,
        mount: Option<Received>,
        cx: &mut Context<Self>,
    ) -> Div {
        let directory = node.kind == NodeKind::Directory;
        let is_shelf = self.folder.is_shelf_entry(node);
        let may_change = self.can_change() && !is_shelf;
        let may_share = self.can_share() && !is_shelf;
        let action =
            |id: &'static str,
             label: &'static str,
             run: fn(&mut Self, &Node, &mut Window, &mut Context<Self>)| {
                let node = node.clone();
                Button::new((id, index))
                    .ghost()
                    .compact()
                    .label(label)
                    .on_click(cx.listener(move |this, _, window, cx| run(this, &node, window, cx)))
            };
        div()
            .flex()
            .flex_row()
            .gap_1()
            .when(directory && !is_shelf, |actions| {
                actions.child(action("zip", "Zip", Self::download_folder))
            })
            .when(!directory, |actions| {
                actions
                    .child(action("download", "Download", Self::download))
                    .child(action("versions", "Versions", Self::versions))
            })
            .when(may_share, |actions| {
                actions.child(action("share", "Share", Self::share))
            })
            .children(mount.map(|mount| {
                Button::new(("leave", index))
                    .ghost()
                    .compact()
                    .label("Leave")
                    .on_click(cx.listener(move |_, _, window, cx| {
                        Self::ask_leave(&mount, window, cx);
                    }))
            }))
            .when(may_change, |actions| {
                actions
                    .child(action("rename", "Rename", |_, node, window, cx| {
                        Self::ask_rename(node, window, cx);
                    }))
                    .child(action("delete", "Delete", Self::ask_remove).danger())
            })
    }

    fn entry(&self, index: usize, node: &Node, cx: &mut Context<Self>) -> impl IntoElement {
        let directory = node.kind == NodeKind::Directory;
        let mount = self.shelf_mount(node);
        let origin = mount
            .as_ref()
            .map(|mount| parts::hint(describe::origin(mount), cx));
        let opened = node.clone();
        parts::row(cx)
            .id(("entry", index))
            .child(
                div()
                    .id(("open", index))
                    .flex_1()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.theme().secondary))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open(&opened, window, cx)),
                    )
                    .child(div().w(px(18.)).child(if directory { "▸" } else { "·" }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .when(directory, |name| name.font_weight(FontWeight::SEMIBOLD))
                                    .child(node.name.clone()),
                            )
                            .children(origin),
                    ),
            )
            .child(
                div()
                    .w(px(80.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if directory {
                        String::new()
                    } else {
                        describe::size(node.size)
                    }),
            )
            .child(
                div()
                    .w(px(110.))
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(describe::day(node.updated_at)),
            )
            .child(self.actions(index, node, mount, cx))
    }

    fn listing(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.listing {
            Load::Loading => parts::state("Loading this folder", cx).into_any_element(),
            Load::Failed(error) => div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(parts::state("This folder did not load.", cx))
                .child(parts::hint(error.clone(), cx))
                .child(
                    Button::new("listing-retry")
                        .label("Try again")
                        .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
                )
                .into_any_element(),
            Load::Ready(nodes) if nodes.is_empty() => {
                let text = if self.can_change() {
                    "This folder is empty. Drop files here, or use the upload button."
                } else {
                    "This folder is empty."
                };
                parts::state(text, cx).into_any_element()
            }
            Load::Ready(nodes) => div()
                .flex()
                .flex_col()
                .children(
                    nodes
                        .iter()
                        .enumerate()
                        .map(|(index, node)| self.entry(index, node, cx).into_any_element())
                        .collect::<Vec<_>>(),
                )
                .into_any_element(),
        }
    }
}

impl Render for FileBrowser {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_change = self.can_change();
        div()
            .id("files")
            .size_full()
            .overflow_y_scroll()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .when(can_change, |area| {
                let target = cx.theme().drop_target;
                area.drag_over::<ExternalPaths>(move |style, _, _, _| {
                    style.border_2().border_color(target).rounded_md()
                })
                .on_drop(cx.listener(
                    |this, paths: &ExternalPaths, window, cx| {
                        this.upload(paths.paths().to_vec(), window, cx);
                    },
                ))
            })
            .child(self.toolbar(cx))
            .children(self.search.clone())
            .children(self.shares.clone())
            .children(self.trash.clone())
            .child(self.banners(cx))
            .child(self.listing(cx))
    }
}
