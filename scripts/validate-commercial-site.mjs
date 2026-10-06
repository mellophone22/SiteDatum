import { readFile, access } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const site = resolve(root, 'site');
const pages = [
  'index.html',
  'early-access.html',
  'support.html',
  'privacy.html',
  'terms.html',
  'refunds.html',
  'release-notes.html',
  'system-requirements.html',
];
const failures = [];

const redirects = await readFile(resolve(site, '.htaccess'), 'utf8');
for (const required of [
  'RewriteCond %{HTTP_HOST} ^www\\.sitedatum\\.site$ [NC]',
  'RewriteRule ^ https://sitedatum.site%{REQUEST_URI} [R=301,L,NE]',
]) {
  if (!redirects.includes(required)) failures.push(`.htaccess: missing canonical redirect rule: ${required}`);
}

for (const page of pages) {
  const source = await readFile(resolve(site, page), 'utf8');
  for (const required of ['<meta name="viewport"', '<main', 'site.css?v=20261006-2', 'data-nav-toggle', 'class="footer-brand-block"', 'release-notes.html', 'system-requirements.html']) {
    if (!source.includes(required)) failures.push(`${page}: missing ${required}`);
  }
  for (const match of source.matchAll(/(?:href|src)="([^"]+)"/g)) {
    const target = match[1];
    if (/^(?:https?:|#|mailto:)/.test(target)) continue;
    const localTarget = target.split(/[?#]/)[0];
    if (!localTarget) continue;
    try { await access(resolve(site, localTarget)); }
    catch { failures.push(`${page}: broken local reference ${target}`); }
  }
}

const index = await readFile(resolve(site, 'index.html'), 'utf8');
for (const required of ['Unsigned Windows Early Access', '$15 monthly or $150 annually', 'No account required', 'Actual SiteDatum 1.4.1 application screen', 'Start of day', 'During the day', 'End of day']) {
  if (!index.includes(required)) failures.push(`index.html: missing approved statement: ${required}`);
}
for (const removed of ['Actual SiteDatum 1.4.0 application screen', '08:10', '11:45', '16:20', 'Reduced motion replaces movement']) {
  if (index.includes(removed)) failures.push(`index.html: obsolete or internal-facing copy remains: ${removed}`);
}

const earlyAccess = await readFile(resolve(site, 'early-access.html'), 'utf8');
for (const required of [
  'before purchasing or downloading',
  'may block installation completely',
  'Do not disable Microsoft Defender',
  'SiteDatum_1.4.1_x64-setup.exe',
  'CAB68C74EDBE1F8C79780C709701ECA52FE1C711240D3CDE2D186E6062CA28A7',
  'a9ab136d43de9777dbf970c3b6ddef091baebb90',
  'Authenticode',
  'NotSigned',
  'https://sitedatum.site/downloads/SiteDatum_1.4.1_x64-setup.exe',
  'Get-FileHash -Algorithm SHA256',
  'supportsitedatum@protonmail.com',
]) {
  if (!earlyAccess.includes(required)) failures.push(`early-access.html: missing safety disclosure: ${required}`);
}

const checkout = await readFile(resolve(root, 'supabase/functions/_shared/checkout_session.ts'), 'utf8');
if (!checkout.includes('managed_payments[enabled]')) failures.push('checkout_session.ts: Managed Payments must remain enabled');
if (checkout.includes('custom_text[')) failures.push('checkout_session.ts: Managed Payments rejects custom_text parameters');

const support = await readFile(resolve(site, 'support.html'), 'utf8');
for (const required of [
  'mailto:supportsitedatum@protonmail.com',
  'billing questions',
  'suspected security or privacy problems',
  'Do not email passwords',
  'github.com/mellophone22/SiteDatum/issues/new?template=bug.md',
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
    'Windows Early Access',
    'without a trusted Windows publisher signature',
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
    'Unsigned-installer compatibility',
    'cannot safely install the build',
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

const releaseNotes = await readFile(resolve(site, 'release-notes.html'), 'utf8');
for (const required of ['SiteDatum 1.4.1', 'October 3, 2026', 'CAB68C74EDBE1F8C79780C709701ECA52FE1C711240D3CDE2D186E6062CA28A7', 'NotSigned']) {
  if (!releaseNotes.includes(required)) failures.push(`release-notes.html: missing verified release detail: ${required}`);
}

const requirements = await readFile(resolve(site, 'system-requirements.html'), 'utf8');
for (const required of ['Windows 10 or 11', '64-bit', 'Microsoft Edge WebView2', '760 × 540', '200% Windows display scaling', 'unsigned Windows Early Access']) {
  if (!requirements.includes(required)) failures.push(`system-requirements.html: missing requirement: ${required}`);
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Commercial site validation passed (${pages.length} pages).`);
}
