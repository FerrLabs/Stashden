import type { account as english } from '../en/account';
import type { Mirror } from '../message';

export const account = {
  'password.heading': 'Modifier votre mot de passe',
  'password.freshSignIn':
    'Vous vous connectez avec votre fournisseur d’identité. Reconnectez-vous pour confirmer votre identité, puis revenez ici pour définir un mot de passe.',
  'password.current': 'Mot de passe actuel',
  'password.new': 'Nouveau mot de passe',
  'password.hint': 'Au moins douze caractères. Vos autres appareils restent connectés.',
  'password.submit': 'Modifier le mot de passe',
  'password.busy': 'Modification en cours',

  'deleteOwn.heading': 'Supprimer votre compte',
  'deleteOwn.hint':
    'Cela met fin à toutes les sessions et supprime le compte. Si vous ne confiez vos fichiers à personne, ils seront supprimés à la fin du délai de grâce.',
  'deleteOwn.freshSignIn':
    'Vous vous connectez avec votre fournisseur d’identité. Reconnectez-vous pour confirmer votre identité, puis revenez ici.',
  'deleteOwn.heir': 'Personne à qui confier vos fichiers (facultatif)',
  'deleteOwn.heirPlaceholder': 'collegue@example.com',
  'deleteOwn.password': 'Votre mot de passe',
  'deleteOwn.address': 'Saisissez votre adresse e-mail pour confirmer',
  'deleteOwn.submit': 'Supprimer mon compte',
  'deleteOwn.busy': 'Suppression en cours',

  'appPasswords.heading': 'Mots de passe d’application',
  'appPasswords.hint':
    'Un client WebDAV ou la commande de synchronisation se connecte avec l’un d’eux, jamais avec le mot de passe que vous utilisez ici. Chacun peut être révoqué séparément.',
  'appPasswords.onlyOnce':
    'Ce mot de passe n’est affiché qu’une seule fois. Le serveur n’en garde qu’une empreinte et ne peut pas le réafficher.',
  'appPasswords.client': 'À renseigner dans le client',
  'appPasswords.address': 'Adresse',
  'appPasswords.userName': 'Nom d’utilisateur',
  'appPasswords.theOneAbove': 'celui ci-dessus',
  'appPasswords.loading': 'Chargement de vos mots de passe d’application',
  'appPasswords.loadFailed': 'Vos mots de passe d’application n’ont pas pu être chargés.',
  'appPasswords.none': 'Vous n’en avez ajouté aucun.',
  'appPasswords.revokeLabel': 'Révoquer {name}',
  'appPasswords.addFor': 'En ajouter un pour',
  'appPasswords.placeholder': 'Ordinateur portable, téléphone, Finder',
  'appPasswords.add': 'Ajouter',
  'appPasswords.adding': 'Ajout en cours',
  'appPasswords.revokeHeading': 'Révoquer {name} ?',
  'appPasswords.revokeBody':
    'Les appareils qui l’utilisent ne pourront plus se connecter. Vos autres mots de passe d’application continuent de fonctionner.',
  'appPasswords.neverUsed': 'Ajouté le {added}, jamais utilisé',
  'appPasswords.lastUsed': 'Ajouté le {added}, dernière utilisation le {used}',
} as const satisfies Mirror<typeof english>;
