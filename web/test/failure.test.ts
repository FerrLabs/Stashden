import assert from 'node:assert/strict';
import { test } from 'node:test';
import { stubBrowser } from './browser.ts';

stubBrowser('en');
const { chooseLocale } = await import('../src/app/i18n/language.ts');
const { failureText, signInFailure } = await import('../src/app/failure.ts');
const { RequestFailed } = await import('../src/app/request-failed.ts');

const LANGUAGES = [
  { locale: 'en', invalid: 'Wrong email or password.', unreachable: 'The server is not responding. Try again later.' },
  {
    locale: 'fr',
    invalid: 'Adresse e-mail ou mot de passe incorrect.',
    unreachable: 'Le serveur ne répond pas. Réessayez plus tard.',
  },
] as const;

for (const { locale, invalid, unreachable } of LANGUAGES) {
  test(`${locale}: a 401 on sign-in shows the translated message without a prefix`, () => {
    chooseLocale(locale);
    assert.equal(signInFailure(new RequestFailed(401, 'invalid email or password')), invalid);
  });

  test(`${locale}: a network failure shows the translated message, not the TypeError`, () => {
    chooseLocale(locale);
    const offline = new TypeError('Failed to fetch');
    assert.equal(signInFailure(offline), unreachable);
    assert.equal(failureText(offline), unreachable);
  });

  test(`${locale}: a failed preview shows the message without the Error prefix`, () => {
    chooseLocale(locale);
    const refused = failureText(new RequestFailed(500, 'storage is down'));
    assert.equal(refused, 'storage is down');
    assert.doesNotMatch(refused, /^(Type)?Error:/);
  });
}

test('other sign-in failures keep the server message', () => {
  chooseLocale('en');
  assert.equal(signInFailure(new RequestFailed(429, 'too many attempts')), 'too many attempts');
});

test('a value that is not an error is rendered as text', () => {
  assert.equal(failureText('boom'), 'boom');
});
