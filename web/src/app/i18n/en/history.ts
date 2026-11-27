export const history = {
  'trash.heading': 'Trash',
  'trash.empty': 'Empty the trash',
  'trash.loading': 'Loading the trash',
  'trash.loadFailed': 'The trash did not load.',
  'trash.isEmpty': 'The trash is empty.',
  'trash.restoreLabel': 'Restore {name}',
  'trash.purge': 'Delete forever',
  'trash.purgeLabel': 'Delete {name} for good',
  'trash.deleted': 'deleted {moment}',
  'trash.expires': 'goes for good {moment}',
  'trash.restoreConflict': '{reason}. Rename or move that one, then restore again.',
  'trash.announce.emptied': 'Emptied the trash',
  'trash.announce.restored': 'Restored {name} to where it was',
  'trash.announce.purged': 'Deleted {name} for good',
  'trash.emptyHeading': 'Empty the trash?',
  'trash.emptyBody':
    'Everything in it is deleted for good, with its versions. This cannot be undone.',
  'trash.purgeHeading': 'Delete {name} for good?',
  'trash.purgeFolder':
    'The folder and everything in it are deleted, with their versions. This cannot be undone.',
  'trash.purgeFile': 'The file is deleted, with its versions. This cannot be undone.',

  'versions.heading': 'Versions of {name}',
  'versions.hint':
    'Each time the file is replaced, what it held before is kept here. The oldest go first when the list is full or the account runs out of room.',
  'versions.loading': 'Loading the versions',
  'versions.loadFailed': 'The versions did not load.',
  'versions.none': 'No earlier versions. This file has not been replaced yet.',
  'versions.downloadLabel': 'Download the version from {moment}',
  'versions.restoreLabel': 'Restore the version from {moment}',
  'versions.restoreHeading': 'Restore {name}?',
  'versions.restoreBody':
    'It goes back to the version from {moment}. The current content is kept as a version, so this can be undone.',
  'versions.announce.downloaded': 'Downloaded the version from {moment}.',
  'versions.announce.saved': 'Saved the version from {moment} to {destination}.',
  'versions.announce.restored':
    'Restored the version from {moment}. What it replaced is now a version too.',
} as const;
