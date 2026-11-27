import { computed, signal, type Signal } from '@angular/core';

export type Step = 'next' | 'previous' | 'first' | 'last';

export function neighbour(ids: readonly string[], from: string | null, step: Step): string | null {
  if (ids.length === 0) {
    return null;
  }
  const at = from === null ? -1 : ids.indexOf(from);
  switch (step) {
    case 'first':
      return ids[0] ?? null;
    case 'last':
      return ids.at(-1) ?? null;
    case 'next':
      return ids[Math.min(at + 1, ids.length - 1)] ?? null;
    case 'previous':
      return ids[Math.max(at - 1, 0)] ?? null;
  }
}

export function successor(ids: readonly string[], removed: string): string | null {
  const at = ids.indexOf(removed);
  if (at < 0) {
    return null;
  }
  return ids[at + 1] ?? ids[at - 1] ?? null;
}

export class RowSelection {
  private readonly picked = signal<string | null>(null);
  private readonly cursor = signal<string | null>(null);

  readonly selected = computed(() => {
    const id = this.picked();
    return id !== null && this.ids().includes(id) ? id : null;
  });

  readonly tabStop = computed(() => {
    const ids = this.ids();
    const at = this.cursor();
    if (at !== null && ids.includes(at)) {
      return at;
    }
    return this.selected() ?? ids[0] ?? null;
  });

  private readonly ids: Signal<readonly string[]>;

  constructor(ids: Signal<readonly string[]>) {
    this.ids = ids;
  }

  select(id: string): void {
    this.picked.set(id);
    this.cursor.set(id);
  }

  focus(id: string): void {
    this.cursor.set(id);
  }

  move(step: Step): string | null {
    const id = neighbour(this.ids(), this.tabStop(), step);
    if (id !== null) {
      this.select(id);
    }
    return id;
  }

  clear(): void {
    this.picked.set(null);
  }

  reset(): void {
    this.picked.set(null);
    this.cursor.set(null);
  }

  passFrom(id: string): void {
    if (this.selected() !== id) {
      return;
    }
    const next = successor(this.ids(), id);
    this.picked.set(next);
    this.cursor.set(next);
  }
}
