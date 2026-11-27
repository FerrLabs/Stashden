import type { publicLink as english } from '../en/public';
import type { Mirror } from '../message';

export const publicLink = {
  'public.needsPassword': 'Ce lien nécessite un mot de passe',
  'public.open': 'Ouvrir',
  'public.wrongPassword': 'Ce mot de passe n’ouvre pas ce lien.',
  'public.opening': 'Ouverture du lien',
  'public.busy.heading': 'Trop de tentatives',
  'public.busy.detail': {
    one: 'Patientez environ {count} seconde, puis réessayez.',
    other: 'Patientez environ {count} secondes, puis réessayez.',
  },
  'public.gone.heading': 'Ce lien n’est plus disponible',
  'public.gone.detail': 'Il a peut-être expiré, ou la personne qui l’a partagé l’a retiré.',
  'public.folderZip': 'Télécharger ce dossier en zip',
  'public.trail': 'Dossiers parents de celui-ci',

  'public.error.needsPassword': 'Ce lien nécessite son mot de passe',
  'public.error.busy': 'Trop de tentatives sur ce lien',

  'reset.ask.heading': 'Réinitialiser votre mot de passe',
  'reset.ask.detail':
    'Saisissez l’adresse avec laquelle vous vous connectez, et nous vous enverrons un lien pour choisir un nouveau mot de passe.',
  'reset.ask.submit': 'Envoyer le lien',
  'reset.ask.busy': 'Envoi en cours',
  'reset.mailed.heading': 'Consultez votre messagerie',
  'reset.mailed.detail':
    'Si un compte se connecte avec {email}, un lien pour réinitialiser son mot de passe est en route. Il ne fonctionne qu’une fois, dans l’heure qui suit.',
  'reset.choose.heading': 'Choisissez un nouveau mot de passe',
  'reset.choose.hint':
    'Au moins douze caractères. Tous les appareils connectés à ce compte sont déconnectés.',
  'reset.choose.submit': 'Enregistrer le mot de passe',
  'reset.choose.busy': 'Enregistrement en cours',
  'reset.changed.heading': 'Mot de passe modifié',
  'reset.changed.detail': 'Connectez-vous avec votre nouveau mot de passe.',
  'reset.back': 'Retour à la connexion',
  'reset.tooMany': {
    one: 'Trop de requêtes, réessayez dans environ {count} seconde.',
    other: 'Trop de requêtes, réessayez dans environ {count} secondes.',
  },

  'error.unreachable': 'Le serveur n’a pas répondu, réessayez plus tard.',
  'error.readOnly': 'ce compte est en lecture seule',
  'error.noRoom': 'il n’y a plus de place dans ce compte',
  'error.notCurrentPassword': 'ce n’est pas votre mot de passe actuel',
  'error.notPassword': 'ce n’est pas votre mot de passe',
  'error.notAddress': 'ce n’est pas votre adresse e-mail',
  'error.connectionLost': 'la connexion au serveur a été perdue',
  'error.serverAnswered': 'le serveur a répondu {status}',
} as const satisfies Mirror<typeof english>;
