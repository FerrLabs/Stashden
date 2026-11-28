import {
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  untracked,
  type Signal,
} from '@angular/core';
import { Location } from '@angular/common';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router, convertToParamMap } from '@angular/router';
import { linkTo } from '../folder';
import type { Node } from '../node';
import { OPEN_PARAM, targetOf } from './open-link';

export type OpenedFileOptions = {
  readonly path: Signal<string>;
  readonly nodes: Signal<readonly Node[]>;
  readonly resolved: Signal<boolean>;
  readonly opened: (node: Node) => void;
  readonly closed: () => void;
  readonly missing: (name: string) => void;
};

export type Leaving = {
  readonly back: () => void;
  readonly replace: () => void;
};

export type Arrival = 'list' | 'link';

const EXIT = { list: 'back', link: 'replace' } as const satisfies Record<Arrival, keyof Leaving>;

export function closeFile(arrival: Arrival, leaving: Leaving): void {
  leaving[EXIT[arrival]]();
}

export class OpenedFile {
  private readonly router = inject(Router);
  private readonly injector = inject(Injector);
  private readonly location = inject(Location);
  private readonly query = toSignal(inject(ActivatedRoute).queryParamMap, {
    initialValue: convertToParamMap({}),
  });
  private readonly name = computed(() => this.query().get(OPEN_PARAM));
  private arrival: Arrival = 'link';
  private wasOpen = false;
  private readonly options: OpenedFileOptions;

  readonly file = computed(() => {
    const name = this.name();
    const target = name === null ? null : targetOf(name, this.options.nodes());
    return target?.kind === 'file' ? target.node : null;
  });

  constructor(options: OpenedFileOptions) {
    this.options = options;
    effect(() => this.resolve());
    effect(() => this.follow());
  }

  open(node: Node): void {
    this.arrival = 'list';
    void this.router.navigate(linkTo(this.options.path()), {
      queryParams: { [OPEN_PARAM]: node.name },
    });
  }

  close(): void {
    closeFile(this.arrival, {
      back: () => this.location.back(),
      replace: () => void this.router.navigate(linkTo(this.options.path()), { replaceUrl: true }),
    });
  }

  private resolve(): void {
    const name = this.name();
    if (name === null || !this.options.resolved()) {
      return;
    }
    const target = targetOf(name, this.options.nodes());
    if (target.kind === 'directory') {
      const folder = linkTo(this.options.path());
      void this.router.navigate([...folder, target.node.name], { replaceUrl: true });
      return;
    }
    if (target.kind === 'missing') {
      this.options.missing(name);
      void this.router.navigate(linkTo(this.options.path()), { replaceUrl: true });
    }
  }

  private follow(): void {
    const node = this.file();
    if (node !== null) {
      untracked(() => this.options.opened(node));
    } else if (this.wasOpen) {
      this.arrival = 'link';
      afterNextRender(() => this.options.closed(), { injector: this.injector });
    }
    this.wasOpen = node !== null;
  }
}
