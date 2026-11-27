import type { sharing as english } from '../en/sharing';
import type { Mirror } from '../message';

export const sharing = {
  'access.read': 'lecture seule',
  'access.write': 'peut modifier',

  'share.link.heading': 'Lien vers {name}',
  'share.link.onlyOnce':
    'Ce lien n’est affiché qu’une seule fois. Le serveur n’en garde qu’une empreinte et ne peut pas le réafficher.',
  'share.link.passwordNote':
    'Toute personne qui l’ouvre devra saisir le mot de passe que vous avez défini.',
  'share.link.copyFailed': 'La copie a échoué. Sélectionnez le lien et copiez-le à la main.',
  'share.heading': 'Partager {name}',
  'share.mode.label': 'Mode de partage',
  'share.mode.link': 'Par lien',
  'share.mode.account': 'Avec un compte',
  'share.link.folderHint':
    'Toute personne disposant du lien peut parcourir et télécharger ce dossier sans compte.',
  'share.link.fileHint':
    'Toute personne disposant du lien peut télécharger ce fichier sans compte.',
  'share.expiry.label': 'Expire le',
  'share.expiry.hint': 'Laissez vide pour un lien qui n’expire pas.',
  'share.expiry.invalid': 'Cette date d’expiration n’est pas une date que le navigateur comprend.',
  'share.password.hint': 'Facultatif, au moins six caractères. Il ne fait pas partie du lien.',
  'share.link.create': 'Créer le lien',
  'share.link.creating': 'Création en cours',

  'share.account.hint':
    'Le destinataire le retrouve dans « {shelf} ». Une adresse qui n’a pas encore de compte le recevra lorsqu’un compte sera créé pour elle.',
  'share.account.access': 'Le destinataire peut',
  'share.account.read': 'Consulter et télécharger',
  'share.account.write': 'Modifier, ajouter et supprimer',
  'share.account.writeHint':
    'Ce que le destinataire ajoute ou modifie est stocké dans vos fichiers et compte dans votre espace de stockage.',
  'share.account.people': 'Partagé avec',
  'share.account.removeLabel': 'Ne plus partager avec {email}',
  'share.account.remove': 'Retirer',
  'share.account.submit': 'Partager',
  'share.account.busy': 'Partage en cours',
  'share.account.shared': 'Partagé avec {email}, {access}',
  'share.account.withdrawn': '{email} n’a plus accès',
  'share.account.stopHeading': 'Ne plus partager {name} avec {email} ?',
  'share.stopBody':
    'Il disparaît de leur dossier « {shelf} » dès leur prochaine requête. Vos fichiers ne sont pas touchés.',

  'share.panel.links': 'Liens que vous avez publiés',
  'share.panel.loadingLinks': 'Chargement de vos liens',
  'share.panel.linksFailed': 'Vos liens n’ont pas pu être chargés.',
  'share.panel.noLinks': 'Vous n’avez publié aucun lien.',
  'share.panel.revokeLabel': 'Révoquer le lien vers {name}',
  'share.panel.accounts': 'Partagé avec d’autres comptes',
  'share.panel.loadingAccounts': 'Chargement de ce que vous partagez',
  'share.panel.accountsFailed': 'Ce que vous partagez n’a pas pu être chargé.',
  'share.panel.noAccounts': 'Vous ne partagez rien avec un autre compte.',
  'share.panel.stopLabel': 'Ne plus partager {name} avec {email}',
  'share.panel.revokeHeading': 'Révoquer le lien vers {name} ?',
  'share.panel.revokeBody':
    'Toute personne qui le détient ne peut plus l’ouvrir. Le fichier lui-même n’est pas touché.',
  'share.panel.inTrash': 'dans la corbeille',

  'share.describe.none': 'Sans expiration',
  'share.describe.unreadable': 'Expiration illisible',
  'share.describe.expired': 'Expiré le {date}',
  'share.describe.expires': 'Expire le {date}',
  'share.describe.never': 'Jamais ouvert',
  'share.describe.lastOpened': 'Dernière ouverture le {date}',
  'share.describe.password': 'Protégé par mot de passe',
} as const satisfies Mirror<typeof english>;
