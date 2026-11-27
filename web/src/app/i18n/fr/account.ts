import type { account as english } from '../en/account';
import type { Mirror } from '../message';

export const account = {
  'password.heading': 'Changer votre mot de passe',
  'password.freshSignIn':
    'Votre compte se connecte via votre fournisseur d’identité. Reconnectez-vous pour confirmer votre identité, puis revenez ici pour définir un mot de passe.',
  'password.current': 'Mot de passe actuel',
  'password.new': 'Nouveau mot de passe',
  'password.hint':
    'Au moins douze caractères. Vos sessions sur d’autres appareils ne sont pas déconnectées.',
  'password.submit': 'Changer le mot de passe',
  'password.busy': 'Modification en cours',

  'deleteOwn.heading': 'Supprimer votre compte',
  'deleteOwn.hint':
    'Cela met fin à toutes les sessions et supprime le compte. Sans personne à qui confier vos fichiers, ils sont libérés puis supprimés à l’issue du délai de grâce.',
  'deleteOwn.freshSignIn':
    'Votre compte se connecte via votre fournisseur d’identité. Reconnectez-vous pour confirmer votre identité, puis revenez ici.',
  'deleteOwn.heir': 'Confier vos fichiers à (facultatif)',
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
  'appPasswords.placeholder': 'Portable, téléphone, Finder',
  'appPasswords.add': 'Ajouter',
  'appPasswords.adding': 'Ajout en cours',
  'appPasswords.revokeHeading': 'Révoquer {name} ?',
  'appPasswords.revokeBody':
    'Tout ce qui se connecte avec lui cesse de fonctionner dès sa prochaine requête. Vos autres mots de passe d’application ne sont pas touchés.',
  'appPasswords.neverUsed': 'Ajouté le {added}, jamais utilisé',
  'appPasswords.lastUsed': 'Ajouté le {added}, dernière utilisation le {used}',
} as const satisfies Mirror<typeof english>;
