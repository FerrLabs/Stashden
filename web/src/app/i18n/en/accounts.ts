export const accounts = {
  'accounts.heading': 'Accounts',
  'accounts.add': 'Add an account',
  'accounts.loading': 'Loading the accounts',
  'accounts.loadFailed': 'The accounts did not load.',
  'accounts.you': 'you',
  'accounts.disabled': 'disabled',
  'accounts.roleOf': 'Role of {email}',
  'accounts.quota': 'Quota',
  'accounts.password': 'Password',
  'accounts.unlock': 'Unlock',
  'accounts.disable': 'Disable',
  'accounts.enable': 'Enable',
  'accounts.nothingStored': 'Nothing stored yet',
  'accounts.usage': '{used} of {max}',

  'accounts.reset.heading': 'Reset the password of {email}',
  'accounts.reset.action': 'Reset',
  'accounts.disable.heading': 'Disable {email}?',
  'accounts.disable.body':
    'Their sessions stop working on their next request, and they cannot sign in again until you enable them. Their files are untouched.',

  'accounts.announce.role': '{email} is now {role}',
  'accounts.announce.disabled': '{email} is disabled',
  'accounts.announce.enabled': '{email} can sign in again',
  'accounts.announce.deleted': '{email} is deleted',
  'accounts.announce.unlocked': '{email} may try again',
  'accounts.announce.quota': 'The quota of {email} is set',
  'accounts.passwordFor.reset': 'New password for {email}',
  'accounts.passwordFor.created': 'Password for {email}',

  'accounts.failed.change': 'Changing {email} failed: {reason}',
  'accounts.failed.delete': 'Deleting {email} failed: {reason}',
  'accounts.failed.unlock': 'Unlocking {email} failed: {reason}',
  'accounts.failed.quota': 'Changing the quota of {email} failed: {reason}',
  'accounts.failed.reset': 'Resetting the password of {email} failed: {reason}',
  'accounts.failed.create': 'Creating {email} failed: {reason}',

  'accounts.new.heading': 'Add an account',
  'accounts.new.displayName': 'Display name',
  'accounts.new.role': 'Role',
  'accounts.new.firstPassword': 'First password',
  'accounts.new.hint':
    'Generated here, and yours to change. It is shown once more after the account is made, and never again.',
  'accounts.new.submit': 'Add',

  'accounts.quota.heading': 'Quota for {email}',
  'accounts.quota.gigabytes': 'Gigabytes',
  'accounts.quota.hint':
    'What the account may hold. Lowering it below what is already stored refuses the next upload rather than deleting anything.',
  'accounts.quota.submit': 'Set',
  'accounts.quota.invalid': 'A quota is a number of gigabytes above zero.',

  'accounts.delete.heading': 'Delete {email}?',
  'accounts.delete.files': 'Their files',
  'accounts.delete.withAccount': 'Delete them with the account',
  'accounts.delete.handTo': 'Hand them to {name} ({email})',
  'accounts.delete.hint':
    'They hold {usage}. Handing them over puts everything in a folder named after this account in the other one’s files, and counts against that account’s quota. Their trash is emptied either way, and the links and shares they made stop working. This cannot be undone.',
  'accounts.delete.submit': 'Delete the account',

  'accounts.password.onlyOnce':
    'This is the only time it is shown. The server keeps only a hash of it, so hand it over now.',
  'accounts.password.hint':
    'Generated here, and yours to change. At least twelve characters. It is shown once more after it is set, and never again.',
} as const;
