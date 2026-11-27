export const sharing = {
  'access.read': 'view only',
  'access.write': 'can edit',

  'share.link.heading': 'Link to {name}',
  'share.link.onlyOnce':
    'This is the only time the link is shown. The server keeps a fingerprint of it and cannot show it again.',
  'share.link.passwordNote': 'Whoever opens it will be asked for the password you set.',
  'share.link.copyFailed': 'Copying failed. Select the link and copy it by hand.',
  'share.heading': 'Share {name}',
  'share.mode.label': 'How to share',
  'share.mode.link': 'With a link',
  'share.mode.account': 'With an account',
  'share.link.folderHint':
    'Anyone holding the link can browse and download this folder without an account.',
  'share.link.fileHint': 'Anyone holding the link can download this file without an account.',
  'share.expiry.label': 'Expiry date',
  'share.expiry.hint': 'Leave it empty for a link that does not expire.',
  'share.expiry.invalid': 'Invalid date. Pick one from the calendar.',
  'share.password.hint': 'Optional, and at least six characters. It is not part of the link.',
  'share.link.create': 'Create link',
  'share.link.creating': 'Creating',

  'share.account.hint':
    'They find it under {shelf}. An address with no account yet receives it when one is made for it.',
  'share.account.access': 'They can',
  'share.account.read': 'View and download',
  'share.account.write': 'Edit, add and delete',
  'share.account.writeHint':
    'What they add or change is stored in your files and counts against your storage.',
  'share.account.people': 'Shared with',
  'share.account.removeLabel': 'Stop sharing with {email}',
  'share.account.remove': 'Remove',
  'share.account.submit': 'Share',
  'share.account.busy': 'Sharing',
  'share.account.shared': 'Shared with {email}, {access}',
  'share.account.withdrawn': '{email} no longer has access',
  'share.account.stopHeading': 'Stop sharing {name} with {email}?',
  'share.stopBody':
    'It will disappear from their {shelf}. Your files are kept.',

  'share.panel.links': 'Links you have published',
  'share.panel.loadingLinks': 'Loading your links',
  'share.panel.linksFailed': 'Your links did not load.',
  'share.panel.noLinks': 'You have not published any links.',
  'share.panel.revokeLabel': 'Revoke the link to {name}',
  'share.panel.accounts': 'Shared with other accounts',
  'share.panel.loadingAccounts': 'Loading what you share',
  'share.panel.accountsFailed': 'What you share did not load.',
  'share.panel.noAccounts': 'You do not share anything with another account.',
  'share.panel.stopLabel': 'Stop sharing {name} with {email}',
  'share.panel.revokeHeading': 'Revoke the link to {name}?',
  'share.panel.revokeBody':
    'Anyone who has the link will no longer be able to open it. The file or folder is kept.',
  'share.panel.inTrash': 'in the trash',

  'share.describe.none': 'No expiry',
  'share.describe.unreadable': 'Expiry unreadable',
  'share.describe.expired': 'Expired {date}',
  'share.describe.expires': 'Expires {date}',
  'share.describe.never': 'Never opened',
  'share.describe.lastOpened': 'Last opened {date}',
  'share.describe.password': 'Password protected',
} as const;
