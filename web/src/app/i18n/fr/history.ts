import type { history as english } from '../en/history';
import type { Mirror } from '../message';

export const history = {
  'trash.heading': 'Corbeille',
  'trash.empty': 'Vider la corbeille',
  'trash.loading': 'Chargement de la corbeille',
  'trash.loadFailed': 'La corbeille n’a pas pu être chargée.',
  'trash.isEmpty': 'La corbeille est vide.',
  'trash.restoreLabel': 'Restaurer {name}',
  'trash.purge': 'Supprimer définitivement',
  'trash.purgeLabel': 'Supprimer {name} définitivement',
  'trash.deleted': 'supprimé le {moment}',
  'trash.expires': 'disparaît définitivement le {moment}',
  'trash.restoreConflict': '{reason}. Renommez ou déplacez cet élément, puis restaurez à nouveau.',
  'trash.announce.emptied': 'Corbeille vidée',
  'trash.announce.restored': '{name} restauré à son emplacement d’origine',
  'trash.announce.purged': '{name} supprimé définitivement',
  'trash.emptyHeading': 'Vider la corbeille ?',
  'trash.emptyBody':
    'Tout son contenu est supprimé définitivement, avec ses versions. Cette action est irréversible.',
  'trash.purgeHeading': 'Supprimer {name} définitivement ?',
  'trash.purgeFolder':
    'Le dossier et tout ce qu’il contient sont supprimés, avec leurs versions. Cette action est irréversible.',
  'trash.purgeFile': 'Le fichier est supprimé, avec ses versions. Cette action est irréversible.',

  'versions.heading': 'Versions de {name}',
  'versions.hint':
    'Chaque fois que le fichier est remplacé, son contenu précédent est conservé ici. Les plus anciennes versions disparaissent en premier lorsque la liste est pleine ou que le compte manque de place.',
  'versions.loading': 'Chargement des versions',
  'versions.loadFailed': 'Les versions n’ont pas pu être chargées.',
  'versions.none': 'Aucune version antérieure. Ce fichier n’a pas encore été remplacé.',
  'versions.downloadLabel': 'Télécharger la version du {moment}',
  'versions.restoreLabel': 'Restaurer la version du {moment}',
  'versions.restoreHeading': 'Restaurer {name} ?',
  'versions.restoreBody':
    'Le fichier revient à la version du {moment}. Le contenu actuel est conservé comme version, ce qui permet d’annuler.',
  'versions.announce.downloaded': 'Version du {moment} téléchargée.',
  'versions.announce.saved': 'Version du {moment} enregistrée dans {destination}.',
  'versions.announce.restored':
    'Version du {moment} restaurée. Le contenu qu’elle a remplacé est désormais une version à son tour.',
} as const satisfies Mirror<typeof english>;
