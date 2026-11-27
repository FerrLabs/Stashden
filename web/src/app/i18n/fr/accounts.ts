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
    'Ses sessions cessent de fonctionner dès leur prochaine requête, et le compte ne peut plus se connecter tant que vous ne l’avez pas réactivé. Ses fichiers sont conservés.',

  'accounts.announce.role': '{email} a maintenant le rôle {role}',
  'accounts.announce.disabled': 'Compte {email} désactivé',
  'accounts.announce.enabled': '{email} peut de nouveau se connecter',
  'accounts.announce.deleted': 'Compte {email} supprimé',
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
    'Généré automatiquement, vous pouvez le modifier. Il sera affiché une dernière fois après la création du compte, puis plus jamais.',
  'accounts.new.submit': 'Ajouter',

  'accounts.quota.heading': 'Quota de {email}',
  'accounts.quota.gigabytes': 'Gigaoctets',
  'accounts.quota.hint':
    'Espace maximal du compte. S’il passe sous l’espace déjà utilisé, rien n’est supprimé : les envois suivants sont refusés.',
  'accounts.quota.submit': 'Définir',
  'accounts.quota.invalid': 'Saisissez un nombre de gigaoctets supérieur à zéro.',

  'accounts.delete.heading': 'Supprimer {email} ?',
  'accounts.delete.files': 'Ses fichiers',
  'accounts.delete.withAccount': 'Les supprimer avec le compte',
  'accounts.delete.handTo': 'Les confier à {name} ({email})',
  'accounts.delete.hint':
    'Ils occupent {usage}. Si vous les confiez, ils sont placés dans un dossier au nom de ce compte, dans les fichiers de l’autre compte, et décomptés de son quota. Dans les deux cas, la corbeille de ce compte est vidée et les liens et partages qu’il a créés cessent de fonctionner. Cette action est irréversible.',
  'accounts.delete.submit': 'Supprimer le compte',

  'accounts.password.onlyOnce':
    'Il n’est affiché qu’une seule fois. Le serveur n’en garde qu’un hachage : transmettez-le dès maintenant.',
  'accounts.password.hint':
    'Généré automatiquement, vous pouvez le modifier. Au moins douze caractères. Il sera affiché une dernière fois après avoir été défini, puis plus jamais.',
} as const satisfies Mirror<typeof english>;
