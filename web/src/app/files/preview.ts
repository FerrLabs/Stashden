import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { DomSanitizer, type SafeResourceUrl } from '@angular/platform-browser';
import { failureText } from '../failure';
import type { Node } from '../node';
import { PLATFORM } from '../platform';
import { mimeOf, openerFor } from './openers';
import { formatSize, t } from '../i18n/language';

type Shown =
  | { kind: 'loading' }
  | { kind: 'image'; url: SafeResourceUrl }
  | { kind: 'text'; body: string }
  | { kind: 'pdf'; url: SafeResourceUrl }
  | { kind: 'none'; reason: string }
  | { kind: 'failed'; reason: string };

let nextTitle = 0;

@Component({
  selector: 'rx-preview',
  templateUrl: './preview.html',
  styleUrl: './preview.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Preview {
  protected readonly t = t;
  readonly node = input.required<Node>();
  readonly path = input.required<string>();
  readonly closed = output<void>();
  readonly downloaded = output<void>();

  private readonly platform = inject(PLATFORM);
  private readonly sanitizer = inject(DomSanitizer);
  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('dialog');
  private objectUrl: string | null = null;

  protected readonly shown = signal<Shown>({ kind: 'loading' });
  protected readonly size = formatSize;

  protected readonly loading = computed(() => this.shown().kind === 'loading');
  protected readonly image = computed(() => {
    const shown = this.shown();
    return shown.kind === 'image' ? shown.url : null;
  });
  protected readonly pdf = computed(() => {
    const shown = this.shown();
    return shown.kind === 'pdf' ? shown.url : null;
  });
  protected readonly text = computed(() => {
    const shown = this.shown();
    return shown.kind === 'text' ? shown.body : null;
  });
  protected readonly message = computed(() => {
    const shown = this.shown();
    return shown.kind === 'none' || shown.kind === 'failed' ? shown.reason : null;
  });
  protected readonly failed = computed(() => this.shown().kind === 'failed');
  protected readonly titleId = `preview-title-${nextTitle++}`;

  constructor() {
    afterNextRender(() => {
      this.dialog().nativeElement.showModal();
      void this.load();
    });
    inject(DestroyRef).onDestroy(() => this.revoke());
  }

  protected close(): void {
    this.dialog().nativeElement.close();
    this.closed.emit();
  }

  protected async load(): Promise<void> {
    const node = this.node();
    this.shown.set({ kind: 'loading' });
    const { viewer, reason } = openerFor(node, { canChange: false });
    if (viewer === null) {
      this.shown.set({
        kind: 'none',
        reason:
          reason === 'tooLarge'
            ? t('files.preview.tooLarge', { size: formatSize(node.size) })
            : t('files.preview.none'),
      });
      return;
    }

    try {
      const blob = await this.platform.read(this.path());
      if (viewer === 'text') {
        this.shown.set({ kind: 'text', body: await blob.text() });
        return;
      }
      this.objectUrl = URL.createObjectURL(blob.slice(0, blob.size, mimeOf(node.name)));
      this.shown.set({
        kind: viewer,
        url: this.sanitizer.bypassSecurityTrustResourceUrl(this.objectUrl),
      });
    } catch (cause: unknown) {
      this.shown.set({ kind: 'failed', reason: failureText(cause) });
    }
  }

  private revoke(): void {
    if (this.objectUrl !== null) {
      URL.revokeObjectURL(this.objectUrl);
      this.objectUrl = null;
    }
  }
}
