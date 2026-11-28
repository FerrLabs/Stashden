import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { definesFrom, contentSecurityPolicy, withContentSecurityPolicy } from './csp.mjs';

const args = process.argv.slice(2);
const angular = JSON.parse(readFileSync(new URL('../angular.json', import.meta.url), 'utf8'));
const configured = angular.projects.web.architect.build.options.define;
const defines = definesFrom(args, Object.fromEntries(
  Object.entries(configured).map(([key, value]) => [key, value.replace(/^'(.*)'$/s, '$1')]),
));

const ng = fileURLToPath(new URL('../node_modules/.bin/ng', import.meta.url));

const policy = contentSecurityPolicy({
  apiUrl: defines.STASHDEN_API_URL,
  telemetryUrl: defines.STASHDEN_TELEMETRY_URL,
});

const built = spawnSync(ng, ['build', ...args], { stdio: 'inherit' });
if (built.error) {
  console.error(`could not run ng: ${built.error.message}`);
  process.exit(1);
}
if (built.status !== 0) process.exit(built.status ?? 1);

const index = new URL('../dist/index.html', import.meta.url);
writeFileSync(index, withContentSecurityPolicy(readFileSync(index, 'utf8'), policy));
