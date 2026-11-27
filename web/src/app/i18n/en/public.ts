export const publicLink = {
  'public.needsPassword': 'This link needs a password',
  'public.open': 'Open',
  'public.wrongPassword': 'That password did not open this link.',
  'public.opening': 'Opening this link',
  'public.busy.heading': 'Too many attempts',
  'public.busy.detail': {
    one: 'Wait about {count} second and try again.',
    other: 'Wait about {count} seconds and try again.',
  },
  'public.gone.heading': 'This link is no longer available',
  'public.gone.detail': 'It may have expired, or whoever shared it has taken it down.',
  'public.folderZip': 'Download this folder as a zip',
  'public.trail': 'Folders above this one',

  'public.error.needsPassword': 'This link needs its password',
  'public.error.busy': 'Too many attempts on this link',

  'reset.ask.heading': 'Reset your password',
  'reset.ask.detail':
    'Enter the address you sign in with and we will mail you a link to choose a new password.',
  'reset.ask.submit': 'Send the link',
  'reset.ask.busy': 'Sending',
  'reset.mailed.heading': 'Check your mail',
  'reset.mailed.detail':
    'If an account signs in with {email}, a link to reset its password is on its way. It works once, within the hour.',
  'reset.choose.heading': 'Choose a new password',
  'reset.choose.hint':
    'At least twelve characters. Every device signed in to this account is signed out.',
  'reset.choose.submit': 'Save the password',
  'reset.choose.busy': 'Saving',
  'reset.changed.heading': 'Password changed',
  'reset.changed.detail': 'Sign in with your new password.',
  'reset.back': 'Back to sign in',
  'reset.tooMany': {
    one: 'Too many requests, try again in about {count} second.',
    other: 'Too many requests, try again in about {count} seconds.',
  },

  'error.unreachable': 'The server did not answer, try again later.',
  'error.readOnly': 'this account may only read',
  'error.noRoom': 'there is no room left in this account',
  'error.notCurrentPassword': 'that is not your current password',
  'error.notPassword': 'that is not your password',
  'error.notAddress': 'that is not your address',
  'error.connectionLost': 'the connection to the server was lost',
  'error.serverAnswered': 'the server answered {status}',
} as const;
