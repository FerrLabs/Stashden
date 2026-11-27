export const account = {
  'password.heading': 'Change your password',
  'password.freshSignIn':
    'Your account signs in through your identity provider. Sign in again to confirm it is you, then come back here to set a password.',
  'password.current': 'Current password',
  'password.new': 'New password',
  'password.hint': 'At least twelve characters. Your sessions elsewhere are not signed out.',
  'password.submit': 'Change password',
  'password.busy': 'Changing',

  'deleteOwn.heading': 'Delete your account',
  'deleteOwn.hint':
    'This ends every session and removes the account. Without somebody to hand your files to, they are released and collected after the grace period.',
  'deleteOwn.freshSignIn':
    'Your account signs in through your identity provider. Sign in again to confirm it is you, then come back here.',
  'deleteOwn.heir': 'Hand your files to (optional)',
  'deleteOwn.heirPlaceholder': 'colleague@example.com',
  'deleteOwn.password': 'Your password',
  'deleteOwn.address': 'Type your address to confirm',
  'deleteOwn.submit': 'Delete my account',
  'deleteOwn.busy': 'Deleting',

  'appPasswords.heading': 'App passwords',
  'appPasswords.hint':
    'A WebDAV client or the sync command signs in with one of these, never with the password you use here. Each one can be revoked on its own.',
  'appPasswords.onlyOnce':
    'This is the only time the password is shown. The server keeps a fingerprint of it and cannot show it again.',
  'appPasswords.client': 'What to give the client',
  'appPasswords.address': 'Address',
  'appPasswords.userName': 'User name',
  'appPasswords.theOneAbove': 'the one above',
  'appPasswords.loading': 'Loading your app passwords',
  'appPasswords.loadFailed': 'Your app passwords did not load.',
  'appPasswords.none': 'You have not added any.',
  'appPasswords.revokeLabel': 'Revoke {name}',
  'appPasswords.addFor': 'Add one, for',
  'appPasswords.placeholder': 'Laptop, phone, Finder',
  'appPasswords.add': 'Add',
  'appPasswords.adding': 'Adding',
  'appPasswords.revokeHeading': 'Revoke {name}?',
  'appPasswords.revokeBody':
    'Whatever signs in with it stops working on its next request. Your other app passwords are untouched.',
  'appPasswords.neverUsed': 'Added {added}, never used',
  'appPasswords.lastUsed': 'Added {added}, last used {used}',
} as const;
