import type { shell as english } from '../en/shell';
import type { Mirror } from '../message';

export const shell = {
  'app.source': 'Code source',
  'app.signIn.heading': 'Connexion',
  'app.displayName.heading': 'Nom affiché',
  'app.displayName.label': 'Nom',
  'app.error.provider': 'La connexion avec le fournisseur d’identité a échoué : {reason}',
  'app.error.account': 'Votre compte n’a pas pu être chargé : {reason}',
  'app.error.signOut': 'Déconnecté ici, mais {reason}',
  'app.error.displayName': 'Votre nom affiché n’a pas été modifié : {reason}',
  'app.notice.accountDeleted': 'Votre compte a été supprimé.',
  'app.notice.displayName': 'Votre nom affiché a été modifié.',
  'app.notice.password': 'Votre mot de passe a été modifié.',

  'signIn.email': 'Adresse e-mail',
  'signIn.password': 'Mot de passe',
  'signIn.submit': 'Se connecter',
  'signIn.busy': 'Connexion en cours',
  'signIn.forgot': 'Mot de passe oublié ?',
  'signIn.provider': 'Se connecter avec votre fournisseur d’identité',
  'signIn.invalid': 'Adresse e-mail ou mot de passe incorrect.',

  'language.label': 'Langue',

  'menu.displayName': 'Modifier le nom affiché',
  'menu.password': 'Modifier le mot de passe',
  'menu.appPasswords': 'Mots de passe d’application',
  'menu.accounts': 'Comptes',
  'menu.deleteAccount': 'Supprimer le compte',
  'menu.signOut': 'Se déconnecter',

  'role.admin': 'Administrateur',
  'role.member': 'Membre',
  'role.reader': 'Lecture seule',
} as const satisfies Mirror<typeof english>;
