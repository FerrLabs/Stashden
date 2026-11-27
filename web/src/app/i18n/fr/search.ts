import type { search as english } from '../en/search';
import type { Mirror } from '../message';

export const search = {
  'search.heading': 'Recherche',
  'search.label': 'Partie d’un nom',
  'search.placeholder': 'Partie d’un nom de fichier ou de dossier',
  'search.busy': 'Recherche en cours',
  'search.none': 'Aucun fichier ni dossier ne correspond à « {term} ».',
  'search.failed': 'La recherche a échoué : {reason}',
  'search.inFolder': 'dans {path}',
  'search.inRoot': 'dans vos fichiers',
} as const satisfies Mirror<typeof english>;
