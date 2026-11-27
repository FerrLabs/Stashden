const CHARSET_META = /<meta charset="UTF-8"\s*\/>/;

export function originOf(url) {
  return url.length > 0 ? new URL(url, 'http://placeholder.invalid').origin : null;
}

export function contentSecurityPolicy({ apiUrl, telemetryUrl }) {
  const connect = new Set(["'self'"]);
  for (const url of [apiUrl, telemetryUrl]) {
    const origin = originOf(url);
    if (origin !== null) connect.add(origin);
  }
  return [
    "default-src 'self'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "object-src 'none'",
    "base-uri 'self'",
    "img-src 'self' blob: data:",
    'frame-src blob: about:',
    "worker-src 'self' blob:",
    `connect-src ${[...connect].join(' ')}`,
    "style-src 'self' 'unsafe-inline'",
  ].join('; ');
}

export function withContentSecurityPolicy(html, policy) {
  if (!CHARSET_META.test(html)) {
    throw new Error('index.html has no charset meta to insert the policy after');
  }
  const meta = `<meta http-equiv="Content-Security-Policy" content="${policy}" />`;
  return html.replace(CHARSET_META, (charset) => `${charset}${meta}`);
}

export function definesFrom(args, fallback) {
  const defines = { ...fallback };
  args.forEach((arg, index) => {
    const value = arg === '--define' ? args[index + 1] : arg.startsWith('--define=') ? arg.slice(9) : null;
    const match = value?.match(/^(\w+)=(.*)$/s);
    if (match) defines[match[1]] = match[2].replace(/^'(.*)'$/s, '$1');
  });
  return defines;
}
