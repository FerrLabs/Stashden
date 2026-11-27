export const files = {
  'files.breadcrumb.label': 'Folder path',
  'files.breadcrumb.root': 'All files',

  'files.search': 'Search',
  'files.sharing': 'Sharing',
  'files.trash': 'Trash',
  'files.newFolder': 'New folder',
  'files.upload.files': 'Upload files',
  'files.upload.folder': 'Upload folder',
  'files.upload.pending': {
    one: 'Uploading {count} file',
    other: 'Uploading {count} files',
  },
  'files.upload.progress': 'Uploading {name}',
  'files.dropHint': 'Drop to upload',

  'files.loading': 'Loading this folder',
  'files.loadFailed': 'This folder did not load.',
  'files.emptyHint': 'Drop files here, or use the upload button.',
  'files.receivedFailed':
    'Who shared this with you, and how, did not load, so nothing here can be changed for now.',
  'files.sharedBy.read':
    '{owner} shares this with you. You can view and download what is in it, and not change it.',
  'files.sharedBy.write':
    '{owner} shares this with you. What you add or change here is stored in their files.',
  'files.origin': 'Shared by {owner}, {access}',

  'files.list.label': 'Files in {folder}',
  'files.list.help': 'Enter opens. Right arrow reaches the actions.',
  'files.open': 'Open',
  'files.open.missing': 'There is no file named {name} in this folder.',
  'files.openWith.preview': 'Preview',

  'files.action.open': 'Open {name}',
  'files.action.openWith': 'Open {name} with',
  'files.action.downloadZip': 'Download {name} as a zip',
  'files.action.versions': 'Versions of {name}',
  'files.action.share': 'Share {name}',
  'files.action.leave': 'Leave {name}',
  'files.action.rename': 'Rename {name}',
  'files.action.delete': 'Delete {name}',
  'files.leave': 'Leave',

  'files.delete.heading': 'Delete {name}?',
  'files.delete.folderOwn':
    'The folder and everything in it move to the trash, where you can restore them.',
  'files.delete.fileOwn': 'The file moves to the trash, where you can restore it.',
  'files.delete.folderShared':
    'The folder and everything in it move to the trash of the account that shares it with you.',
  'files.delete.fileShared': 'The file moves to the trash of the account that shares it with you.',
  'files.leave.heading': 'Leave {name}?',
  'files.leave.body':
    'It disappears from {shelf}. {owner} keeps it, and can share it with you again.',

  'files.rename.heading': 'Rename {name}',
  'files.rename.label': 'New name',
  'files.rename.hint': 'Type a path with / to move it into a folder here.',
  'files.rename.action': 'Rename',
  'files.newFolder.label': 'Name',
  'files.newFolder.hint': 'Type a path with / to make folders inside folders.',

  'files.announce.left': 'Left {name}',
  'files.announce.saved': 'Saved {name} to {destination}',
  'files.announce.downloaded': 'Downloaded {name}',
  'files.announce.savedZip': 'Saved {name}.zip to {destination}',
  'files.announce.downloadedZip': 'Downloaded {name}.zip',
  'files.announce.moved': 'Moved {name} to {destination}',
  'files.announce.renamed': 'Renamed {name}',
  'files.announce.created': 'Created {name}',
  'files.announce.trashed': 'Moved {name} to the trash',
  'files.announce.uploaded': {
    one: 'Uploaded {count} file',
    other: 'Uploaded {count} files',
  },

  'files.failed.leave': 'Leaving {name} failed: {reason}',
  'files.failed.download': 'Downloading {name} failed: {reason}',
  'files.failed.rename': 'Renaming {name} failed: {reason}',
  'files.failed.create': 'Creating {name} failed: {reason}',
  'files.failed.delete': 'Deleting {name} failed: {reason}',
  'files.failed.upload': 'Uploading {name} failed: {reason}',

  'files.preview.close': 'Close the preview',
  'files.preview.loading': 'Loading the file',
  'files.preview.tooLarge': 'This file is {size}, too large to open here.',
  'files.preview.none': 'There is no preview for this kind of file.',
} as const;
