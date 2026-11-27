import type { accounts as english } from '../en/accounts';
import type { Mirror } from '../message';

export const accounts = {
  'accounts.heading': 'Comptes',
  'accounts.add': 'Ajouter un compte',
  'accounts.loading': 'Chargement des comptes',
  'accounts.loadFailed': 'Les comptes n’ont pas pu être chargés.',
  'accounts.you': 'vous',
  'accounts.disabled': 'désactivé',
  'accounts.roleOf': 'Rôle de {email}',
  'accounts.quota': 'Quota',
  'accounts.password': 'Mot de passe',
  'accounts.unlock': 'Déverrouiller',
  'accounts.disable': 'Désactiver',
  'accounts.enable': 'Activer',
  'accounts.nothingStored': 'Rien de stocké pour l’instant',
  'accounts.usage': '{used} sur {max}',

  'accounts.reset.heading': 'Réinitialiser le mot de passe de {email}',
  'accounts.reset.action': 'Réinitialiser',
  'accounts.disable.heading': 'Désactiver {email} ?',
  'accounts.disable.body':
    'Ses sessions cessent de fonctionner dès leur prochaine requête, et le compte ne peut plus se connecter tant que vous ne le réactivez pas. Ses fichiers ne sont pas touchés.',

  'accounts.announce.role': '{email} a maintenant le rôle {role}',
  'accounts.announce.disabled': '{email} est désactivé',
  'accounts.announce.enabled': '{email} peut de nouveau se connecter',
  'accounts.announce.deleted': '{email} est supprimé',
  'accounts.announce.unlocked': '{email} peut réessayer',
  'accounts.announce.quota': 'Le quota de {email} est défini',
  'accounts.passwordFor.reset': 'Nouveau mot de passe pour {email}',
  'accounts.passwordFor.created': 'Mot de passe pour {email}',

  'accounts.failed.change': 'La modification de {email} a échoué : {reason}',
  'accounts.failed.delete': 'La suppression de {email} a échoué : {reason}',
  'accounts.failed.unlock': 'Le déverrouillage de {email} a échoué : {reason}',
  'accounts.failed.quota': 'La modification du quota de {email} a échoué : {reason}',
  'accounts.failed.reset': 'La réinitialisation du mot de passe de {email} a échoué : {reason}',
  'accounts.failed.create': 'La création de {email} a échoué : {reason}',

  'accounts.new.heading': 'Ajouter un compte',
  'accounts.new.displayName': 'Nom affiché',
  'accounts.new.role': 'Rôle',
  'accounts.new.firstPassword': 'Premier mot de passe',
  'accounts.new.hint':
    'Généré ici, et modifiable. Il est affiché une dernière fois après la création du compte, puis plus jamais.',
  'accounts.new.submit': 'Ajouter',

  'accounts.quota.heading': 'Quota de {email}',
  'accounts.quota.gigabytes': 'Gigaoctets',
  'accounts.quota.hint':
    'Ce que le compte peut contenir. Le baisser en dessous de ce qui est déjà stocké refuse le prochain envoi au lieu de supprimer quoi que ce soit.',
  'accounts.quota.submit': 'Définir',
  'accounts.quota.invalid': 'Un quota est un nombre de gigaoctets supérieur à zéro.',

  'accounts.delete.heading': 'Supprimer {email} ?',
  'accounts.delete.files': 'Ses fichiers',
  'accounts.delete.withAccount': 'Les supprimer avec le compte',
  'accounts.delete.handTo': 'Les confier à {name} ({email})',
  'accounts.delete.hint':
    'Ils occupent {usage}. Les confier place tout dans un dossier portant le nom de ce compte, dans les fichiers de l’autre compte, et compte dans son quota. La corbeille de ce compte est vidée dans les deux cas, et les liens et partages qu’il a créés cessent de fonctionner. Cette action est irréversible.',
  'accounts.delete.submit': 'Supprimer le compte',

  'accounts.password.onlyOnce':
    'Il n’est affiché qu’une seule fois. Le serveur n’en garde qu’un hachage : transmettez-le dès maintenant.',
  'accounts.password.hint':
    'Généré ici, et modifiable. Au moins douze caractères. Il est affiché une dernière fois après avoir été défini, puis plus jamais.',
} as const satisfies Mirror<typeof english>;
