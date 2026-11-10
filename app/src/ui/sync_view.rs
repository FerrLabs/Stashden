use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;
use stashden_client::sync::watch::{Command, Status};
use tokio::sync::broadcast::error::RecvError;

use super::parts;
use super::run;
use crate::describe;
use crate::desktop::Desktop;
use crate::sync::Syncing;

pub struct SyncView {
    desktop: Arc<Desktop>,
    _following: Task<()>,
    folders: Vec<Syncing>,
    failure: Option<String>,
    busy: bool,
}

pub fn upsert(folders: &mut Vec<Syncing>, syncing: Syncing) {
    if matches!(syncing.status, Status::Stopped) {
        folders.retain(|one| one.folder != syncing.folder);
    } else if let Some(one) = folders.iter_mut().find(|one| one.folder == syncing.folder) {
        *one = syncing;
    } else {
        folders.push(syncing);
        folders.sort_by(|a, b| a.folder.cmp(&b.folder));
    }
}

impl SyncView {
    pub fn new(desktop: Arc<Desktop>, cx: &mut Context<Self>) -> Self {
        let mut changes = desktop.subscribe();
        let following = cx.spawn(async move |this, cx| {
            loop {
                match changes.recv().await {
                    Ok(syncing) => {
                        if this
                            .update(cx, |view, cx| {
                                upsert(&mut view.folders, syncing);
                                cx.notify();
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        if this.update(cx, |view, cx| view.refresh(cx)).is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });

        let view = Self {
            desktop,
            _following: following,
            folders: Vec::new(),
            failure: None,
            busy: false,
        };
        view.refresh(cx);
        view
    }

    fn refresh(&self, cx: &mut Context<Self>) {
        let desktop = Arc::clone(&self.desktop);
        let snapshot = crate::runtime::spawn(async move { desktop.syncing().await });
        cx.spawn(async move |this, cx| {
            let folders = snapshot.await;
            let _ = this.update(cx, |view, cx| {
                view.folders = folders;
                cx.notify();
            });
        })
        .detach();
    }

    fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let desktop = Arc::clone(&self.desktop);
        self.begin(window, cx, async move {
            let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await else {
                return Ok(());
            };
            desktop.start_sync(folder.path().to_path_buf()).await
        });
    }

    fn control(
        &mut self,
        folder: PathBuf,
        command: Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let desktop = Arc::clone(&self.desktop);
        self.begin(window, cx, async move {
            desktop.control_sync(folder, command).await
        });
    }

    fn begin(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        work: impl Future<Output = Result<(), crate::failure::Failure>> + Send + 'static,
    ) {
        self.failure = None;
        self.busy = true;
        cx.notify();
        run(window, cx, work, |this, outcome, _, cx| {
            this.busy = false;
            this.failure = outcome.err().map(|failure| failure.to_string());
            this.refresh(cx);
            cx.notify();
        });
    }

    fn folder(&self, index: usize, current: &Syncing, cx: &mut Context<Self>) -> impl IntoElement {
        let paused = matches!(current.status, Status::Paused);
        let control = |id: &'static str, label: &'static str, command: Command| {
            let folder = current.folder.clone();
            Button::new((id, index))
                .label(label)
                .disabled(self.busy)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.control(folder.clone(), command, window, cx);
                }))
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(current.folder.display().to_string()),
            )
            .child(div().child(describe::status(&current.status)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        control("sync-now", "Sync now", Command::SyncNow)
                            .primary()
                            .disabled(self.busy || paused),
                    )
                    .child(if paused {
                        control("resume", "Resume", Command::Resume).ghost()
                    } else {
                        control("pause", "Pause", Command::Pause).ghost()
                    })
                    .child(control("stop", "Stop", Command::Stop).ghost()),
            )
            .children(current.last.as_ref().map(|last| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Last pass"))
                    .child(div().child(describe::summary(last)))
                    .children(describe::attention(last).into_iter().map(|(tag, line)| {
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(parts::tag(tag, tag == "failed", cx))
                            .child(line)
                    }))
            }))
    }
}

impl Render for SyncView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = self.folders.is_empty();
        let folders: Vec<_> = self
            .folders
            .clone()
            .iter()
            .enumerate()
            .map(|(index, current)| self.folder(index, current, cx).into_any_element())
            .collect();
        let add = Button::new("sync-add")
            .label(if empty {
                "Choose a folder"
            } else {
                "Add a folder"
            })
            .disabled(self.busy)
            .on_click(cx.listener(|this, _, window, cx| this.choose(window, cx)));

        div()
            .id("sync")
            .size_full()
            .overflow_y_scroll()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(parts::heading("Sync"))
            .children(
                self.failure
                    .clone()
                    .map(|failure| parts::failure(failure, cx)),
            )
            .child(parts::hint(
                "Keep folders on this computer in step with your account. Changes on either side \
                 are sent across as they happen, and nothing is overwritten: when a file changed \
                 in both places, both copies are kept. Syncing carries on from the system tray \
                 when you close the window, and folders start again the next time you open \
                 Stashden.",
                cx,
            ))
            .children(folders)
            .child(div().child(if empty { add.primary() } else { add.ghost() }))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use stashden_client::sync::watch::Status;

    use super::upsert;
    use crate::sync::Syncing;

    fn syncing(folder: &str, status: Status) -> Syncing {
        Syncing {
            folder: PathBuf::from(folder),
            status,
            last: None,
        }
    }

    #[test]
    fn an_update_replaces_its_folder_and_a_new_folder_is_kept_in_order() {
        let mut folders = vec![syncing("/b", Status::Idle)];
        upsert(&mut folders, syncing("/a", Status::Idle));
        upsert(&mut folders, syncing("/b", Status::Syncing));

        assert_eq!(folders.len(), 2);
        assert_eq!(folders[0].folder, PathBuf::from("/a"));
        assert_eq!(folders[1].status, Status::Syncing);
    }

    #[test]
    fn a_stopped_folder_disappears() {
        let mut folders = vec![syncing("/a", Status::Idle), syncing("/b", Status::Idle)];
        upsert(&mut folders, syncing("/a", Status::Stopped));
        assert_eq!(folders, [syncing("/b", Status::Idle)]);
    }
}
