import { execFileSync } from 'node:child_process';
import { Buffer } from 'node:buffer';
import process from 'node:process';

const required = [
  'SITEDATUM_LICENSING_PROJECT_URL',
  'SITEDATUM_LICENSING_PUBLISHABLE_KEY',
  'SITEDATUM_ENTITLEMENT_PUBLIC_KEY_B64',
  'SITEDATUM_ENTITLEMENT_KEY_ID',
];

const failures = [];
for (const name of required) {
  if (!process.env[name]?.trim()) failures.push(`${name} is required`);
}

const projectUrl = process.env.SITEDATUM_LICENSING_PROJECT_URL ?? '';
if (projectUrl === 'https://lirkgkiwbffhsmlrfsbp.supabase.co') {
  failures.push('production build points to the SiteDatum sandbox project');
}
try {
  const parsed = new URL(projectUrl);
  if (parsed.protocol !== 'https:' || !parsed.hostname.endsWith('.supabase.co') || parsed.pathname !== '/') {
    failures.push('SITEDATUM_LICENSING_PROJECT_URL must be an HTTPS Supabase project origin');
  }
} catch {
  failures.push('SITEDATUM_LICENSING_PROJECT_URL is invalid');
}

if (!(process.env.SITEDATUM_LICENSING_PUBLISHABLE_KEY ?? '').startsWith('sb_publishable_')) {
  failures.push('SITEDATUM_LICENSING_PUBLISHABLE_KEY must use a Supabase publishable key');
}

try {
  const publicKey = Buffer.from(process.env.SITEDATUM_ENTITLEMENT_PUBLIC_KEY_B64 ?? '', 'base64');
  if (publicKey.length !== 32) failures.push('SITEDATUM_ENTITLEMENT_PUBLIC_KEY_B64 must decode to 32 bytes');
} catch {
  failures.push('SITEDATUM_ENTITLEMENT_PUBLIC_KEY_B64 is invalid base64');
}

if (/test|sandbox|local|replace/i.test(process.env.SITEDATUM_ENTITLEMENT_KEY_ID ?? '')) {
  failures.push('SITEDATUM_ENTITLEMENT_KEY_ID must identify a production signing key');
}

const status = execFileSync('git', ['status', '--porcelain'], { encoding: 'utf8', windowsHide: true }).trim();
if (status) failures.push('working tree must be clean before a production commerce build');

if (failures.length) {
  console.error('Production commerce build refused:');
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log('Production commerce configuration passed (secret values not displayed).');
