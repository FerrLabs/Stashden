export type RowCommand =
  | { readonly kind: 'move'; readonly step: 'next' | 'previous' | 'first' | 'last' }
  | { readonly kind: 'open' }
  | { readonly kind: 'select' }
  | { readonly kind: 'clear' }
  | { readonly kind: 'actions' }
  | { readonly kind: 'rename' }
  | { readonly kind: 'remove' }
  | { readonly kind: 'menu' };

export type Keystroke = {
  readonly key: string;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
};

export function commandFor(stroke: Keystroke): RowCommand | null {
  if (stroke.altKey || stroke.ctrlKey || stroke.metaKey) {
    return null;
  }
  if (stroke.key === 'F10') {
    return stroke.shiftKey ? { kind: 'menu' } : null;
  }
  if (stroke.shiftKey) {
    return null;
  }
  switch (stroke.key) {
    case 'ArrowDown':
      return { kind: 'move', step: 'next' };
    case 'ArrowUp':
      return { kind: 'move', step: 'previous' };
    case 'Home':
      return { kind: 'move', step: 'first' };
    case 'End':
      return { kind: 'move', step: 'last' };
    case 'Enter':
      return { kind: 'open' };
    case ' ':
      return { kind: 'select' };
    case 'Escape':
      return { kind: 'clear' };
    case 'ArrowRight':
      return { kind: 'actions' };
    case 'F2':
      return { kind: 'rename' };
    case 'Delete':
      return { kind: 'remove' };
    case 'ContextMenu':
      return { kind: 'menu' };
    default:
      return null;
  }
}

export function opensOnTap(pointerType: string): boolean {
  return pointerType === 'touch' || pointerType === 'pen';
}
