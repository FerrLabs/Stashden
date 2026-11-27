import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  input,
  output,
  signal,
  viewChild,
  viewChildren,
} from '@angular/core';
import { t } from '../i18n/language';
import type { Way } from './openers';

@Component({
  selector: 'rx-open-with',
  templateUrl: './open-with.html',
  styleUrl: './open-with.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OpenWith {
  protected readonly t = t;
  readonly name = input.required<string>();
  readonly ways = input.required<readonly Way[]>();
  readonly reachable = input(true);
  readonly chosen = output<Way>();

  private readonly trigger = viewChild.required<ElementRef<HTMLButtonElement>>('trigger');
  private readonly menu = viewChild.required<ElementRef<HTMLElement>>('menu');
  private readonly items = viewChildren<ElementRef<HTMLButtonElement>>('item');

  protected readonly expanded = signal(false);

  open(first = true): void {
    const menu = this.menu().nativeElement;
    const trigger = this.trigger().nativeElement;
    if (menu.matches(':popover-open') || trigger.getClientRects().length === 0) {
      return;
    }
    const box = trigger.getBoundingClientRect();
    menu.style.top = `${box.bottom + 4}px`;
    menu.style.left = `${box.left}px`;
    menu.showPopover();
    const items = this.items();
    const target = first ? items.at(0) : items.at(-1);
    target?.nativeElement.focus();
  }

  protected toggle(): void {
    const menu = this.menu().nativeElement;
    if (menu.matches(':popover-open')) {
      menu.hidePopover();
      return;
    }
    this.open();
  }

  protected onTriggerKey(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      this.open(event.key === 'ArrowDown');
    }
  }

  protected onMenuKey(event: KeyboardEvent): void {
    event.stopPropagation();
    const items = this.items().map((item) => item.nativeElement);
    const at = items.findIndex((item) => item === document.activeElement);
    const last = items.length - 1;
    switch (event.key) {
      case 'ArrowDown':
        items.at(at >= last ? 0 : at + 1)?.focus();
        break;
      case 'ArrowUp':
        items.at(at <= 0 ? last : at - 1)?.focus();
        break;
      case 'Home':
        items.at(0)?.focus();
        break;
      case 'End':
        items.at(-1)?.focus();
        break;
      case 'Tab':
        this.menu().nativeElement.hidePopover();
        return;
      default:
        return;
    }
    event.preventDefault();
  }

  protected choose(way: Way): void {
    this.menu().nativeElement.hidePopover();
    this.chosen.emit(way);
  }

  protected onToggle(event: Event): void {
    this.expanded.set(event instanceof ToggleEvent && event.newState === 'open');
  }
}
