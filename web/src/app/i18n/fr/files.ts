import type { files as english } from '../en/files';
import type { Mirror } from '../message';

export const files = {
  'files.breadcrumb.label': 'Chemin du dossier',
  'files.breadcrumb.root': 'Tous les fichiers',

  'files.search': 'Rechercher',
  'files.sharing': 'Partage',
  'files.trash': 'Corbeille',
  'files.newFolder': 'Nouveau dossier',
  'files.upload.files': 'Envoyer des fichiers',
  'files.upload.folder': 'Envoyer un dossier',
  'files.upload.pending': {
    one: 'Envoi de {count} fichier en cours',
    other: 'Envoi de {count} fichiers en cours',
  },
  'files.upload.progress': 'Envoi de {name}',
  'files.dropHint': 'Déposez pour envoyer',

  'files.loading': 'Chargement du dossier',
  'files.loadFailed': 'Ce dossier n’a pas pu être chargé.',
  'files.emptyHint': 'Déposez des fichiers ici, ou utilisez le bouton d’envoi.',
  'files.receivedFailed':
    'Impossible de savoir qui a partagé ce dossier avec vous, ni avec quels droits. Rien ne peut être modifié ici pour le moment.',
  'files.sharedBy.read':
    '{owner} partage ceci avec vous. Vous pouvez consulter et télécharger son contenu, mais pas le modifier.',
  'files.sharedBy.write':
    '{owner} partage ceci avec vous. Ce que vous ajoutez ou modifiez ici est enregistré dans ses fichiers.',
  'files.origin': 'Partagé par {owner}, {access}',

  'files.list.label': 'Fichiers de {folder}',
  'files.list.labelRoot': 'Fichiers',
  'files.list.help': 'Entrée ouvre. La flèche droite mène aux actions.',
  'files.open': 'Ouvrir',
  'files.open.missing': 'Il n’y a aucun fichier nommé « {name} » dans ce dossier.',
  'files.openWith.preview': 'Aperçu',

  'files.action.open': 'Ouvrir {name}',
  'files.action.openWith': 'Ouvrir {name} avec',
  'files.action.downloadZip': 'Télécharger {name} en zip',
  'files.action.versions': 'Versions de {name}',
  'files.action.share': 'Partager {name}',
  'files.action.leave': 'Quitter {name}',
  'files.action.rename': 'Renommer {name}',
  'files.action.delete': 'Supprimer {name}',
  'files.leave': 'Quitter',

  'files.delete.heading': 'Supprimer {name} ?',
  'files.delete.folderOwn':
    'Le dossier et tout ce qu’il contient sont placés dans la corbeille, d’où vous pourrez les restaurer.',
  'files.delete.fileOwn': 'Le fichier est placé dans la corbeille, d’où vous pourrez le restaurer.',
  'files.delete.folderShared':
    'Le dossier et tout ce qu’il contient sont placés dans la corbeille du compte qui le partage avec vous.',
  'files.delete.fileShared':
    'Le fichier est placé dans la corbeille du compte qui le partage avec vous.',
  'files.leave.heading': 'Quitter {name} ?',
  'files.leave.body':
    'Il disparaît de « {shelf} ». {owner} le conserve et peut le partager de nouveau avec vous.',

  'files.rename.heading': 'Renommer {name}',
  'files.rename.label': 'Nouveau nom',
  'files.rename.hint': 'Pour le déplacer dans un sous-dossier, saisissez un chemin avec /.',
  'files.rename.action': 'Renommer',
  'files.newFolder.label': 'Nom',
  'files.newFolder.hint': 'Saisissez un chemin avec / pour créer des dossiers imbriqués.',

  'files.announce.left': 'Vous avez quitté {name}',
  'files.announce.saved': '{name} enregistré dans {destination}',
  'files.announce.downloaded': '{name} téléchargé',
  'files.announce.savedZip': '{name}.zip enregistré dans {destination}',
  'files.announce.downloadedZip': '{name}.zip téléchargé',
  'files.announce.moved': '{name} déplacé vers {destination}',
  'files.announce.renamed': '{name} renommé',
  'files.announce.created': '{name} créé',
  'files.announce.trashed': '{name} placé dans la corbeille',
  'files.announce.uploaded': {
    one: '{count} fichier envoyé',
    other: '{count} fichiers envoyés',
  },

  'files.failed.leave': 'Impossible de quitter {name} : {reason}',
  'files.failed.download': 'Le téléchargement de {name} a échoué : {reason}',
  'files.failed.rename': 'Le renommage de {name} a échoué : {reason}',
  'files.failed.create': 'La création de {name} a échoué : {reason}',
  'files.failed.delete': 'La suppression de {name} a échoué : {reason}',
  'files.failed.upload': 'L’envoi de {name} a échoué : {reason}',

  'files.preview.close': 'Fermer l’aperçu',
  'files.preview.loading': 'Chargement du fichier',
  'files.preview.tooLarge': 'Ce fichier fait {size}, trop volumineux pour être ouvert ici.',
  'files.preview.none': 'Il n’y a pas d’aperçu pour ce type de fichier.',
} as const satisfies Mirror<typeof english>;
