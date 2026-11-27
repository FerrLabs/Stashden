export function stubBrowser(language: string): void {
  const define = (name: string, value: unknown): void => {
    Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
  };
  define('document', { documentElement: { lang: 'en' } });
  define('navigator', { languages: [language] });
  define('localStorage', { getItem: () => null, setItem: () => undefined });
  define('STASHDEN_API_URL', 'https://api.example.test');
}
