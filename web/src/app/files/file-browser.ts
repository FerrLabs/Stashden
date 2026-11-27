import {
  ChangeDetectionStrategy,
  Component,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  resource,
  signal,
  viewChildren,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router, type UrlSegment } from '@angular/router';
import { Session } from '../account';
import { childOf, linkTo } from '../folder';
import { SHARED_WITH_ME, describeOrigin, whereIs, type Received } from '../grant';
import { byKindThenName, type Node } from '../node';
import { chosenFromDrop, fromChosen, type Outgoing } from '../outgoing';
import { PLATFORM } from '../platform';
import { Confirm } from '../shared/confirm';
import { Prompt } from '../shared/prompt';
import { DownloadCancelled } from '../transfer';
import { Breadcrumb } from './breadcrumb';
import { FileRow } from './file-row';
import { loadedNodes } from './open-link';
import { OpenedFile } from './opened-file';
import { removalBody } from './removal-text';
import { Preview } from './preview';
import { RowSelection, type Step } from './selection';
import { SearchPanel } from './search-panel';
import { ShareDialog } from './share-dialog';
import { SharesPanel } from './shares-panel';
import { TrashPanel } from './trash-panel';
import { UploadTarget } from './upload-target';
import { VersionsDialog } from './versions-dialog';
import { formatDate, formatSize, t } from '../i18n/language';

@Component({
  selector: 'rx-file-browser',
  imports: [
    Breadcrumb,
    Confirm,
    FileRow,
    Preview,
    Prompt,
    SearchPanel,
    ShareDialog,
    SharesPanel,
    TrashPanel,
    UploadTarget,
    VersionsDialog,
  ],
  templateUrl: './file-browser.html',
  styleUrl: './file-browser.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FileBrowser {
  protected readonly t = t;
  private readonly platform = inject(PLATFORM);
  private readonly router = inject(Router);

  private readonly segments = toSignal(inject(ActivatedRoute).url, {
    initialValue: [] as UrlSegment[],
  });

  private readonly rows = viewChildren(FileRow);
  private readonly injector = inject(Injector);
  private refocusOnReload = false;

  protected readonly path = computed(() =>
    this.segments()
      .map((segment) => segment.path)
      .join('/'),
  );

  protected readonly listing = resource({
    params: () => ({ path: this.path() }),
    loader: ({ params }) => this.platform.listFolder(`/${params.path}`),
  });

  protected readonly nodes = computed(() => [...loadedNodes(this.listing)].sort(byKindThenName));

  protected readonly selection = new RowSelection(
    computed(() => this.nodes().map((node) => node.id)),
  );
  protected readonly folderName = computed(() => this.segments().at(-1)?.path ?? '');

  private readonly link = new OpenedFile({
    path: this.path,
    nodes: this.nodes,
    resolved: computed(() => this.listing.status() === 'resolved'),
    opened: (node) => this.selection.select(node.id),
    closed: () => this.focusSelected(),
    missing: (name) => this.failure.set(t('files.open.missing', { name })),
  });
  protected readonly opened = this.link.file;

  private readonly session = inject(Session);

  protected readonly canWrite = this.session.canWrite;
  protected readonly where = computed(() => whereIs(this.path()));

  private readonly left = signal(0);
  protected readonly received = resource({
    params: () => (this.where().kind === 'own' ? undefined : { version: this.left() }),
    loader: () => this.platform.receivedGrants?.() ?? Promise.resolve([]),
  });

  protected readonly mount = computed(() => {
    const where = this.where();
    if (where.kind !== 'shared') {
      return null;
    }
    return (this.received.value() ?? []).find((mount) => mount.name === where.mount) ?? null;
  });

  protected readonly canChange = computed(() => {
    const where = this.where();
    const allowed =
      where.kind === 'own' || (where.kind === 'shared' && this.mount()?.access === 'write');
    return this.canWrite() && allowed;
  });
  protected readonly canUpload = computed(
    () => this.platform.upload !== undefined && this.canChange(),
  );
  protected readonly canShare = computed(
    () => this.platform.share !== undefined && this.canWrite() && this.where().kind === 'own',
  );
  protected readonly canLeave = computed(
    () => this.platform.withdrawGrant !== undefined && this.where().kind === 'shelf',
  );
  protected readonly canCreateFolder = computed(
    () => this.platform.createFolder !== undefined && this.canChange(),
  );
  protected readonly canSearch = this.platform.search !== undefined;
  protected readonly canSeeLinks = computed(() => this.platform.listShares !== undefined);
  protected readonly canSeeTrash = this.platform.listTrash !== undefined;
  protected readonly canSeeVersions = this.platform.listVersions !== undefined;
  protected readonly canRestore = computed(
    () => this.platform.restoreVersion !== undefined && this.canChange(),
  );
  protected readonly dragging = signal(false);
  protected readonly pending = signal(0);
  protected readonly sending = signal<{ name: string; sent: number; total: number } | null>(null);
  protected readonly announcement = signal<string | null>(null);
  protected readonly failure = signal<string | null>(null);
  protected readonly doomed = signal<Node | null>(null);
  protected readonly leaving = signal<Received | null>(null);
  protected readonly renaming = signal<Node | null>(null);
  protected readonly naming = signal(false);
  protected readonly searching = signal(false);
  protected readonly sharing = signal<Node | null>(null);
  protected readonly versioning = signal<Node | null>(null);
  protected readonly showingLinks = signal(false);
  protected readonly showingTrash = signal(false);
  protected readonly trashed = signal(0);
  protected readonly published = signal(0);

  protected readonly shelf = SHARED_WITH_ME;
  protected readonly removalBody = removalBody;
  protected readonly size = formatSize;
  protected readonly date = formatDate;

  constructor() {
    effect(() => {
      if (this.listing.status() === 'resolved' && this.refocusOnReload) {
        this.refocusOnReload = false;
        afterNextRender(() => this.focusSelected(), { injector: this.injector });
      }
    });
    effect(() => {
      this.path();
      this.announcement.set(null);
      this.failure.set(null);
      this.selection.reset();
      this.renaming.set(null);
      this.naming.set(false);
      this.sharing.set(null);
      this.versioning.set(null);
    });
  }

  protected isShelf(node: Node): boolean {
    return this.path() === '' && node.kind === 'directory' && node.name === SHARED_WITH_ME;
  }

  protected mayChange(node: Node): boolean {
    return this.canChange() && !this.isShelf(node);
  }

  protected mayShare(node: Node): boolean {
    return this.canShare() && !this.isShelf(node);
  }

  protected mountFor(node: Node): Received | null {
    return (this.received.value() ?? []).find((mount) => mount.name === node.name) ?? null;
  }

  protected originOf(node: Node): string | null {
    if (this.where().kind !== 'shelf') {
      return null;
    }
    const mount = this.mountFor(node);
    return mount === null ? null : describeOrigin(mount);
  }

  protected async leave(mount: Received): Promise<void> {
    this.leaving.set(null);
    const withdraw = this.platform.withdrawGrant;
    if (withdraw === undefined) {
      return;
    }
    await this.attempt(
      (reason) => t('files.failed.leave', { name: mount.name, reason }),
      async () => {
        await withdraw(mount.id);
        this.announcement.set(t('files.announce.left', { name: mount.name }));
        this.left.update((count) => count + 1);
        this.listing.reload();
      },
    );
  }

  protected open(node: Node): void {
    this.selection.select(node.id);
    if (node.kind === 'directory') {
      void this.router.navigate(linkTo(this.pathOf(node)));
      return;
    }
    this.link.open(node);
  }

  protected close(): void {
    this.link.close();
  }

  protected move(step: Step): void {
    const id = this.selection.move(step);
    this.rows()
      .find((row) => row.node().id === id)
      ?.focus();
  }

  protected notePublished(): void {
    this.published.update((count) => count + 1);
  }

  protected pathOf(node: Node): string {
    return childOf(this.path(), node.name);
  }

  protected async download(node: Node): Promise<void> {
    await this.attempt(
      (reason) => t('files.failed.download', { name: node.name, reason }),
      async () => {
        try {
          const saved = await this.platform.download(childOf(this.path(), node.name), node.name);
          this.announcement.set(
            saved
              ? t('files.announce.saved', { name: node.name, destination: saved })
              : t('files.announce.downloaded', { name: node.name }),
          );
        } catch (cause: unknown) {
          if (!(cause instanceof DownloadCancelled)) {
            throw cause;
          }
        }
      },
    );
  }

  protected readonly canArchive = this.platform.downloadArchive !== undefined;

  protected async downloadFolder(node: Node): Promise<void> {
    const archive = this.platform.downloadArchive;
    if (archive === undefined) {
      return;
    }
    await this.attempt(
      (reason) => t('files.failed.download', { name: node.name, reason }),
      async () => {
        try {
          const saved = await archive(childOf(this.path(), node.name), node.name);
          this.announcement.set(
            saved
              ? t('files.announce.savedZip', { name: node.name, destination: saved })
              : t('files.announce.downloadedZip', { name: node.name }),
          );
        } catch (cause: unknown) {
          if (!(cause instanceof DownloadCancelled)) {
            throw cause;
          }
        }
      },
    );
  }

  protected async rename(node: Node, destination: string): Promise<void> {
    this.renaming.set(null);
    const to = childOf(this.path(), destination);
    await this.attempt(
      (reason) => t('files.failed.rename', { name: node.name, reason }),
      async () => {
        await this.platform.rename(this.pathOf(node), to);
        this.announcement.set(
          destination.includes('/')
            ? t('files.announce.moved', { name: node.name, destination: to })
            : t('files.announce.renamed', { name: node.name }),
        );
        this.reloadAndRefocus();
      },
    );
  }

  protected async createFolder(name: string): Promise<void> {
    this.naming.set(false);
    const create = this.platform.createFolder;
    if (create === undefined) {
      return;
    }
    const path = childOf(this.path(), name);
    await this.attempt(
      (reason) => t('files.failed.create', { name, reason }),
      async () => {
        await create(path);
        this.announcement.set(t('files.announce.created', { name }));
        this.listing.reload();
      },
    );
  }

  protected goTo(link: string[]): void {
    this.searching.set(false);
    void this.router.navigate(link);
  }

  protected async remove(node: Node): Promise<void> {
    this.doomed.set(null);
    await this.attempt(
      (reason) => t('files.failed.delete', { name: node.name, reason }),
      async () => {
        await this.platform.remove(childOf(this.path(), node.name));
        this.selection.passFrom(node.id);
        this.announcement.set(t('files.announce.trashed', { name: node.name }));
        this.trashed.update((count) => count + 1);
        this.reloadAndRefocus();
      },
    );
  }

  protected async upload(items: Outgoing[]): Promise<void> {
    this.pending.set(items.length);
    let sent = 0;
    for (const item of items) {
      this.sending.set({ name: item.name, sent: 0, total: 0 });
      const done = await this.attempt(
        (reason) => t('files.failed.upload', { name: item.name, reason }),
        async () => {
          await item.send(childOf(this.path(), item.name), (progress, total) =>
            this.sending.set({ name: item.name, sent: progress, total }),
          );
        },
      );
      if (done) {
        sent += 1;
      }
      this.pending.update((left) => left - 1);
    }
    this.sending.set(null);

    if (sent > 0) {
      this.announcement.set(t('files.announce.uploaded', { count: sent }));
      this.listing.reload();
    }
  }

  protected onDragOver(event: DragEvent): void {
    if (!this.canUpload()) {
      return;
    }
    event.preventDefault();
    this.dragging.set(true);
  }

  protected onDragLeave(): void {
    this.dragging.set(false);
  }

  protected onDrop(event: DragEvent): void {
    if (!this.canUpload()) {
      return;
    }
    event.preventDefault();
    this.dragging.set(false);
    const transfer = event.dataTransfer;
    const upload = this.platform.upload;
    if (transfer === null || upload === undefined) {
      return;
    }
    void chosenFromDrop(transfer).then((chosen) => {
      if (chosen.length > 0) {
        void this.upload(fromChosen(upload, chosen));
      }
    });
  }

  private reloadAndRefocus(): void {
    this.refocusOnReload = true;
    this.listing.reload();
  }

  private focusSelected(): void {
    const id = this.selection.tabStop();
    this.rows()
      .find((row) => row.node().id === id)
      ?.focus();
  }

  private async attempt(
    failed: (reason: string) => string,
    run: () => Promise<void>,
  ): Promise<boolean> {
    try {
      await run();
      return true;
    } catch (cause: unknown) {
      this.announcement.set(null);
      this.failure.set(failed(reasonFor(cause)));
      return false;
    }
  }
}

function reasonFor(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
