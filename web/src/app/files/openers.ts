import type { Node } from '../node';

export type Viewer = 'image' | 'pdf' | 'text';

export type Editor = 'markdown' | 'csv' | 'spreadsheet';

export type Reason = 'ready' | 'tooLarge' | 'unsupported';

export type Opening = {
  readonly viewer: Viewer | null;
  readonly editor: Editor | null;
  readonly reason: Reason;
};

export type Way = 'preview' | 'download';

export type Capabilities = { readonly canChange: boolean };

type Opener = {
  readonly extensions: readonly string[];
  readonly viewer: Viewer | null;
  readonly editor: Editor | null;
  readonly mime?: string;
};

export const PREVIEW_LIMIT = 16 * 1000 * 1000;

const OPENERS: readonly Opener[] = [
  { extensions: ['apng'], viewer: 'image', editor: null, mime: 'image/apng' },
  { extensions: ['avif'], viewer: 'image', editor: null, mime: 'image/avif' },
  { extensions: ['bmp'], viewer: 'image', editor: null, mime: 'image/bmp' },
  { extensions: ['gif'], viewer: 'image', editor: null, mime: 'image/gif' },
  { extensions: ['ico'], viewer: 'image', editor: null, mime: 'image/x-icon' },
  { extensions: ['jpeg', 'jpg'], viewer: 'image', editor: null, mime: 'image/jpeg' },
  { extensions: ['png'], viewer: 'image', editor: null, mime: 'image/png' },
  { extensions: ['svg'], viewer: 'image', editor: null, mime: 'image/svg+xml' },
  { extensions: ['webp'], viewer: 'image', editor: null, mime: 'image/webp' },
  { extensions: ['pdf'], viewer: 'pdf', editor: null, mime: 'application/pdf' },
  { extensions: ['md', 'markdown'], viewer: 'text', editor: null },
  { extensions: ['csv'], viewer: 'text', editor: null },
  { extensions: ['xlsx'], viewer: null, editor: null },
  {
    extensions: [
      'css',
      'html',
      'js',
      'json',
      'log',
      'rs',
      'sh',
      'sql',
      'toml',
      'ts',
      'txt',
      'xml',
      'yaml',
      'yml',
    ],
    viewer: 'text',
    editor: null,
  },
];

export function extensionOf(name: string): string {
  const at = name.lastIndexOf('.');
  return at > 0 ? name.slice(at + 1).toLowerCase() : '';
}

function openerOf(name: string): Opener | undefined {
  const extension = extensionOf(name);
  return OPENERS.find((opener) => opener.extensions.includes(extension));
}

export function openerFor(node: Pick<Node, 'name' | 'size'>, options: Capabilities): Opening {
  if (node.size > PREVIEW_LIMIT) {
    return { viewer: null, editor: null, reason: 'tooLarge' };
  }
  const opener = openerOf(node.name);
  if (opener === undefined || opener.viewer === null) {
    return { viewer: null, editor: null, reason: 'unsupported' };
  }
  return {
    viewer: opener.viewer,
    editor: options.canChange ? opener.editor : null,
    reason: 'ready',
  };
}

export function waysToOpen(opening: Opening): Way[] {
  return opening.viewer === null ? [] : ['preview', 'download'];
}

export function mimeOf(name: string): string {
  return openerOf(name)?.mime ?? 'application/octet-stream';
}
