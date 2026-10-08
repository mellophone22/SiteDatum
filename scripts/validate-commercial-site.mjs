import { readFile, access } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const site = resolve(root, 'site');
const pages = [
  'index.html',
  '404.html',
  'early-access.html',
  'support.html',
  'privacy.html',
  'terms.html',
  'refunds.html',
  'release-notes.html',
  'system-requirements.html',
];
const canonicalUrls = {
  'index.html': 'https://sitedatum.site/',
  '404.html': 'https://sitedatum.site/404.html',
  'early-access.html': 'https://sitedatum.site/early-access.html',
  'support.html': 'https://sitedatum.site/support.html',
  'privacy.html': 'https://sitedatum.site/privacy.html',
  'terms.html': 'https://sitedatum.site/terms.html',
  'refunds.html': 'https://sitedatum.site/refunds.html',
  'release-notes.html': 'https://sitedatum.site/release-notes.html',
  'system-requirements.html': 'https://sitedatum.site/system-requirements.html',
};
const failures = [];

function luminance(hex) {
  const channels = hex.replace('#', '').match(/../g).map((value) => Number.parseInt(value, 16) / 255);
  const linear = channels.map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrast(first, second) {
  const a = luminance(first);
  const b = luminance(second);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

function cssBlock(source, selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escaped}\\s*\\{([^}]+)\\}`));
  return match?.[1] ?? '';
}

function cssToken(block, name) {
  return block.match(new RegExp(`--${name}:\\s*(#[0-9a-f]{6})`, 'i'))?.[1];
}

const redirects = await readFile(resolve(site, '.htaccess'), 'utf8');
for (const required of [
  'RewriteCond %{HTTP_HOST} ^www\\.sitedatum\\.site$ [NC]',
  'RewriteRule ^ https://sitedatum.site%{REQUEST_URI} [R=301,L,NE]',
  'ErrorDocument 404 /404.html',
  'Content-Security-Policy',
  "frame-ancestors 'none'",
  'Strict-Transport-Security "max-age=31536000; includeSubDomains"',
  'X-Content-Type-Options "nosniff"',
  'Referrer-Policy "strict-origin-when-cross-origin"',
  'Permissions-Policy',
]) {
  if (!redirects.includes(required)) failures.push(`.htaccess: missing redirect or security-header contract: ${required}`);
}

for (const page of pages) {
  const source = await readFile(resolve(site, page), 'utf8');
  for (const required of ['<meta name="viewport"', '<main', 'site.css?v=20261006-2', 'data-nav-toggle', 'class="footer-brand-block"', 'release-notes.html', 'system-requirements.html']) {
    if (!source.includes(required)) failures.push(`${page}: missing ${required}`);
  }
  for (const required of [
    `<link rel="canonical" href="${canonicalUrls[page]}"`,
    '<meta property="og:title"',
    '<meta property="og:description"',
    '<meta property="og:type" content="website"',
    '<meta property="og:site_name" content="SiteDatum"',
    `<meta property="og:url" content="${canonicalUrls[page]}"`,
    '<meta property="og:image"',
    '<meta name="twitter:card" content="summary_large_image"',
    '<link rel="icon" type="image/png"',
    '<link rel="apple-touch-icon"',
    '<span data-year>2026</span>',
  ]) {
    if (!source.includes(required)) failures.push(`${page}: missing metadata or no-JavaScript fallback: ${required}`);
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

const robots = await readFile(resolve(site, 'robots.txt'), 'utf8');
if (!robots.includes('Sitemap: https://sitedatum.site/sitemap.xml')) failures.push('robots.txt: sitemap declaration is missing');
const sitemap = await readFile(resolve(site, 'sitemap.xml'), 'utf8');
for (const [page, url] of Object.entries(canonicalUrls)) {
  if (page === '404.html') continue;
  if (!sitemap.includes(`<loc>${url}</loc>`)) failures.push(`sitemap.xml: missing ${url}`);
}

const notFound = await readFile(resolve(site, '404.html'), 'utf8');
for (const required of ['Return home', 'Download SiteDatum', 'Visit support', '<meta name="robots" content="noindex"']) {
  if (!notFound.includes(required)) failures.push(`404.html: missing recovery path: ${required}`);
}

const index = await readFile(resolve(site, 'index.html'), 'utf8');
for (const required of ['Unsigned Windows Early Access', '$15 monthly or $150 annually', 'No account required', 'Actual SiteDatum 1.4.4 application screen', 'Start of day', 'During the day', 'End of day']) {
  if (!index.includes(required)) failures.push(`index.html: missing approved statement: ${required}`);
}
for (const required of ['Download and install SiteDatum first', 'Settings &gt; Account &amp; Subscription', 'Download, then upgrade']) {
  if (!index.includes(required)) failures.push(`index.html: missing Pro upgrade guidance: ${required}`);
}
for (const removed of ['Actual SiteDatum 1.4.0 application screen', '08:10', '11:45', '16:20', 'Reduced motion replaces movement']) {
  if (index.includes(removed)) failures.push(`index.html: obsolete or internal-facing copy remains: ${removed}`);
}

const earlyAccess = await readFile(resolve(site, 'early-access.html'), 'utf8');
for (const required of [
  'before purchasing or downloading',
  'may block installation completely',
  'Do not disable Microsoft Defender',
  'SiteDatum_1.4.4_x64-setup.exe',
  '264D91B38AF4C3251FB04CF23193744FF4B7BFC1DDB58892CC4F04480CD633CF',
  'c6dd1f78846555c5acfb7621e6ac4acf780319f4',
  'Authenticode',
  'NotSigned',
  'https://sitedatum.site/downloads/SiteDatum_1.4.4_x64-setup.exe',
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

const effectiveStatuses = new Map([
  ['privacy.html', 'Founder-approved — effective October 8, 2026'],
  ['terms.html', 'Founder-approved — effective October 3, 2026'],
  ['refunds.html', 'Founder-approved — effective October 3, 2026'],
]);

for (const [page, effectiveStatus] of effectiveStatuses) {
  const source = await readFile(resolve(site, page), 'utf8');
  if (!source.includes(effectiveStatus)) failures.push(`${page}: effective founder-approval status is missing`);
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
for (const required of ['SiteDatum 1.4.4', 'October 8, 2026', '264D91B38AF4C3251FB04CF23193744FF4B7BFC1DDB58892CC4F04480CD633CF', 'NotSigned']) {
  if (!releaseNotes.includes(required)) failures.push(`release-notes.html: missing verified release detail: ${required}`);
}

const requirements = await readFile(resolve(site, 'system-requirements.html'), 'utf8');
for (const required of ['Windows 10 or 11', '64-bit', 'Microsoft Edge WebView2', '760 × 540', '200% Windows display scaling', 'unsigned Windows Early Access']) {
  if (!requirements.includes(required)) failures.push(`system-requirements.html: missing requirement: ${required}`);
}

const appCss = await readFile(resolve(root, 'src/design.css'), 'utf8');
const appBaseCss = await readFile(resolve(root, 'src/App.css'), 'utf8');
for (const required of ['background:var(--action-fill)', 'background:var(--action-fill-hover)', 'color:var(--on-action)', 'outline:3px solid var(--focus-ring)', 'color:var(--placeholder)']) {
  if (!appBaseCss.includes(required)) failures.push(`src/App.css: semantic interaction token is not wired: ${required}`);
}
const lightTheme = cssBlock(appCss, ':root');
const darkTheme = cssBlock(appCss, ':root[data-theme="dark"]');
for (const [themeName, block, canvas, inputBackground] of [
  ['light', lightTheme, '#eef2f2', '#ffffff'],
  ['dark', darkTheme, '#111b21', '#14232b'],
]) {
  const actionFill = cssToken(block, 'action-fill');
  const actionHover = cssToken(block, 'action-fill-hover');
  const onAction = cssToken(block, 'on-action');
  const focusRing = cssToken(block, 'focus-ring');
  const placeholder = cssToken(block, 'placeholder');
  for (const [token, value] of Object.entries({ actionFill, actionHover, onAction, focusRing, placeholder })) {
    if (!value) failures.push(`src/design.css: ${themeName} theme is missing a literal ${token} token`);
  }
  if (actionFill && onAction && contrast(actionFill, onAction) < 4.5) failures.push(`src/design.css: ${themeName} action contrast is below WCAG AA`);
  if (actionHover && onAction && contrast(actionHover, onAction) < 4.5) failures.push(`src/design.css: ${themeName} hover action contrast is below WCAG AA`);
  if (focusRing && contrast(focusRing, canvas) < 3) failures.push(`src/design.css: ${themeName} focus ring contrast is below 3:1`);
  if (placeholder && contrast(placeholder, inputBackground) < 4.5) failures.push(`src/design.css: ${themeName} placeholder contrast is below WCAG AA`);
}

const siteCss = await readFile(resolve(site, 'site.css'), 'utf8');
for (const forbidden of ['.proof-note{display:flex;align-items:center;gap:12px;margin:0;color:var(--line);font-size:.72rem', '.proof-note{align-items:flex-start;font-size:.62rem', '.site-footer nav strong{margin-bottom:4px;color:var(--teal);font-size:.72rem']) {
  if (siteCss.includes(forbidden)) failures.push(`site.css: undersized or low-contrast essential label remains: ${forbidden}`);
}
if (contrast('#05656d', '#f3f6f7') < 4.5) failures.push('site.css: footer label color is below WCAG AA');

if (failures.length) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Commercial site validation passed (${pages.length} pages).`);
}
