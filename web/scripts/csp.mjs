const CHARSET_META = /<meta charset="UTF-8"\s*\/>/;

export function originOf(url) {
  if (url.length === 0 || (url.startsWith('/') && !url.startsWith('//'))) return null;
  let parsed;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`"${url}" is not an absolute http(s) URL, a path starting with "/" or empty`);
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    throw new Error(`"${url}" must use http: or https:`);
  }
  return parsed.origin;
}

export function contentSecurityPolicy({ apiUrl, telemetryUrl }) {
  const connect = new Set(["'self'"]);
  for (const url of [apiUrl, telemetryUrl]) {
    const origin = originOf(url);
    if (origin !== null) connect.add(origin);
  }
  return [
    "default-src 'self'",
    "script-src 'self'",
    'object-src blob:',
    "base-uri 'self'",
    "form-action 'none'",
    "img-src 'self' blob:",
    'frame-src blob:',
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
    if (match) defines[match[1]] = match[2].replace(/^(['"])(.*)\1$/s, '$2');
  });
  return defines;
}
