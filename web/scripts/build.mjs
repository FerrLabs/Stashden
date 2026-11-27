import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { definesFrom, contentSecurityPolicy, withContentSecurityPolicy } from './csp.mjs';

const args = process.argv.slice(2);
const built = spawnSync('ng', ['build', ...args], { stdio: 'inherit' });
if (built.status !== 0) process.exit(built.status ?? 1);

const angular = JSON.parse(readFileSync(new URL('../angular.json', import.meta.url), 'utf8'));
const configured = angular.projects.web.architect.build.options.define;
const defines = definesFrom(args, Object.fromEntries(
  Object.entries(configured).map(([key, value]) => [key, value.replace(/^'(.*)'$/s, '$1')]),
));

const index = new URL('../dist/index.html', import.meta.url);
const policy = contentSecurityPolicy({
  apiUrl: defines.STASHDEN_API_URL,
  telemetryUrl: defines.STASHDEN_TELEMETRY_URL,
});
writeFileSync(index, withContentSecurityPolicy(readFileSync(index, 'utf8'), policy));
