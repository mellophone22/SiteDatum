import { readFile, access } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const site = resolve(root, 'site');
const pages = ['index.html', 'support.html', 'privacy.html', 'terms.html', 'refunds.html'];
const failures = [];

for (const page of pages) {
  const source = await readFile(resolve(site, page), 'utf8');
  for (const required of ['<meta name="viewport"', '<main', 'site.css']) {
    if (!source.includes(required)) failures.push(`${page}: missing ${required}`);
  }
  for (const match of source.matchAll(/(?:href|src)="([^"]+)"/g)) {
    const target = match[1];
    if (/^(?:https?:|#|mailto:)/.test(target)) continue;
    const localTarget = target.split('#')[0];
    if (!localTarget) continue;
    try { await access(resolve(site, localTarget)); }
    catch { failures.push(`${page}: broken local reference ${target}`); }
  }
}

const index = await readFile(resolve(site, 'index.html'), 'utf8');
for (const required of ['Public distribution is deferred', '$15 monthly or $150 annually', 'No account required', 'Actual SiteDatum 1.4.0 application screen']) {
  if (!index.includes(required)) failures.push(`index.html: missing approved statement: ${required}`);
}

const support = await readFile(resolve(site, 'support.html'), 'utf8');
for (const required of [
  'mailto:supportsitedatum@protonmail.com',
  'billing questions',
  'suspected security or privacy problems',
  'Do not email passwords',
  'issuable_template=Bug',
]) {
  if (!support.includes(required)) failures.push(`support.html: missing private-intake safeguard: ${required}`);
}

for (const page of ['privacy.html', 'terms.html', 'refunds.html']) {
  const source = await readFile(resolve(site, page), 'utf8');
  if (!source.includes('Founder-approved — effective October 3, 2026')) failures.push(`${page}: effective founder-approval status is missing`);
  if (source.includes('not yet effective') || source.includes('Policy Draft') || source.includes('Terms Draft')) failures.push(`${page}: stale draft language remains`);
}

const legalRequirements = {
  'privacy.html': [
    'Scope and local project data',
    'Information used for Pro licensing',
    'Service providers',
    'Retention and deletion',
    'Choices and requests',
    'supportsitedatum@protonmail.com',
  ],
  'terms.html': [
    'Subscriptions and renewal',
    'Devices and connectivity',
    'Cancellation and expiration',
    'Warranty',
    'Limitation of liability',
    'Disputes and governing law',
    'supportsitedatum@protonmail.com',
  ],
  'refunds.html': [
    'Merchant of record',
    'Refund requests',
    'Effect of a refund',
    'supportsitedatum@protonmail.com',
  ],
};

for (const [page, requiredStatements] of Object.entries(legalRequirements)) {
  const source = await readFile(resolve(site, page), 'utf8');
  for (const required of requiredStatements) {
    if (!source.includes(required)) failures.push(`${page}: missing policy subject: ${required}`);
  }
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Commercial site validation passed (${pages.length} pages).`);
}
