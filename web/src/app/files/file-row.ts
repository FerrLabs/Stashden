import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  input,
  output,
  viewChild,
} from '@angular/core';
import { formatDate, formatSize, t } from '../i18n/language';
import type { Node } from '../node';
import { commandFor, opensOnTap, visible } from './gestures';
import { OpenWith } from './open-with';
import { waysToOpen, openerFor, type Way } from './openers';
import type { Step } from './selection';

let nextRow = 0;

@Component({
  selector: 'tr[rx-file-row]',
  imports: [OpenWith],
  templateUrl: './file-row.html',
  styleUrl: './file-row.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    role: 'row',
    '[attr.aria-selected]': 'selected()',
    '[attr.aria-labelledby]': 'labelledBy',
    '[attr.tabindex]': 'tabStop() ? 0 : -1',
    '[class.selected]': 'selected()',
    '[class.directory]': 'node().kind === "directory"',
    '(click)': 'onClick($event)',
    '(dblclick)': 'onDoubleClick($event)',
    '(keydown)': 'onKeydown($event)',
    '(focus)': 'focused.emit()',
  },
})
export class FileRow {
  protected readonly t = t;
  private readonly key = `file-row-${nextRow++}`;
  protected readonly nameId = `${this.key}-name`;
  protected readonly sizeId = `${this.key}-size`;
  protected readonly dateId = `${this.key}-date`;
  protected readonly labelledBy = [this.nameId, this.sizeId, this.dateId].join(' ');
  protected readonly size = formatSize;
  protected readonly date = formatDate;

  readonly node = input.required<Node>();
  readonly origin = input<string | null>(null);
  readonly selected = input(false);
  readonly tabStop = input(false);
  readonly canArchive = input(false);
  readonly canSeeVersions = input(false);
  readonly mayShare = input(false);
  readonly mayChange = input(false);
  readonly canLeave = input(false);

  readonly picked = output<void>();
  readonly focused = output<void>();
  readonly moved = output<Step>();
  readonly cleared = output<void>();
  readonly opened = output<void>();
  readonly downloaded = output<void>();
  readonly zipped = output<void>();
  readonly versioned = output<void>();
  readonly shared = output<void>();
  readonly left = output<void>();
  readonly renamed = output<void>();
  readonly removed = output<void>();

  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
  private readonly menu = viewChild(OpenWith);

  protected readonly ways = computed(() => {
    const node = this.node();
    return node.kind === 'file' ? waysToOpen(openerFor(node, { canChange: this.mayChange() })) : [];
  });

  focus(): void {
    this.host.focus();
  }

  protected choose(way: Way): void {
    if (way === 'preview') {
      this.opened.emit();
    } else {
      this.downloaded.emit();
    }
  }

  protected onClick(event: MouseEvent): void {
    this.picked.emit();
    if (this.fromActions(event)) {
      return;
    }
    const pointer = event instanceof PointerEvent ? event.pointerType : '';
    if (opensOnTap(pointer)) {
      this.opened.emit();
    }
  }

  protected onDoubleClick(event: MouseEvent): void {
    if (!this.fromActions(event)) {
      this.opened.emit();
    }
  }

  protected onKeydown(event: KeyboardEvent): void {
    if (event.target !== this.host) {
      this.onActionKey(event);
      return;
    }
    const command = commandFor(event);
    if (command === null) {
      return;
    }
    switch (command.kind) {
      case 'move':
        this.moved.emit(command.step);
        break;
      case 'open':
        this.opened.emit();
        break;
      case 'select':
        this.picked.emit();
        break;
      case 'clear':
        this.cleared.emit();
        break;
      case 'actions':
        this.actions().at(0)?.focus();
        break;
      case 'rename':
        if (this.mayChange()) {
          this.renamed.emit();
        }
        break;
      case 'remove':
        if (this.mayChange()) {
          this.removed.emit();
        }
        break;
      case 'menu':
        this.menu()?.open();
        break;
    }
    event.preventDefault();
  }

  private onActionKey(event: KeyboardEvent): void {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') {
      return;
    }
    const actions = this.actions();
    const at = actions.indexOf(document.activeElement as HTMLElement);
    const next = event.key === 'ArrowRight' ? actions.at(at + 1) : actions.at(at - 1);
    if (event.key === 'ArrowLeft' && at <= 0) {
      this.focus();
    } else {
      (next ?? actions.at(-1))?.focus();
    }
    event.preventDefault();
  }

  private actions(): HTMLElement[] {
    return visible([...this.host.querySelectorAll<HTMLElement>('.action')]);
  }

  private fromActions(event: MouseEvent): boolean {
    return event.target instanceof Element && event.target.closest('.actions') !== null;
  }
}
