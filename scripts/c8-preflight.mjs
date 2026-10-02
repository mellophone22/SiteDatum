import { execFileSync } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import process from 'node:process';

const root = resolve(import.meta.dirname, '..');
const full = process.argv.includes('--full');
const checks = [];

function record(id, status, detail) {
  checks.push({ id, status, detail });
  const marker = status === 'pass' ? 'PASS' : status === 'deferred' ? 'DEFERRED' : 'FAIL';
  console.log(`[${marker}] ${id}: ${detail}`);
}

function read(path) {
  return readFileSync(resolve(root, path), 'utf8');
}

function assert(id, condition, passDetail, failDetail) {
  record(id, condition ? 'pass' : 'fail', condition ? passDetail : failDetail);
}

function run(id, executable, args, cwd = root) {
  const started = Date.now();
  try {
    execFileSync(executable, args, { cwd, encoding: 'utf8', stdio: 'pipe', windowsHide: true });
    record(id, 'pass', `completed in ${((Date.now() - started) / 1000).toFixed(1)}s`);
  } catch (error) {
    const output = `${error.stdout ?? ''}\n${error.stderr ?? ''}`.trim().split(/\r?\n/).slice(-8).join('\n');
    record(id, 'fail', output || error.message);
  }
}

const packageJson = JSON.parse(read('package.json'));
const tauriConfig = JSON.parse(read('src-tauri/tauri.conf.json'));
const cargoToml = read('src-tauri/Cargo.toml');
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
assert('C8-VERSION', packageJson.version === tauriConfig.version && packageJson.version === cargoVersion,
  `package, Tauri, and Rust versions agree at ${packageJson.version}`,
  `version mismatch: package=${packageJson.version}, tauri=${tauriConfig.version}, cargo=${cargoVersion ?? 'missing'}`);

const installer = resolve(root, `src-tauri/target/release/bundle/nsis/SiteDatum_${packageJson.version}_x64-setup.exe`);
assert('C8-INSTALLER', existsSync(installer), `internal NSIS candidate exists for ${packageJson.version}`, `missing ${installer}`);

const tracked = execFileSync('git', ['ls-files'], { cwd: root, encoding: 'utf8', windowsHide: true })
  .split(/\r?\n/).filter(Boolean);
const liveTokenPatterns = [
  /\bsk_live_[A-Za-z0-9_-]+/,
  /\brk_live_[A-Za-z0-9_-]+/,
  /\bpk_live_[A-Za-z0-9_-]+/,
  /\bwhsec_[A-Za-z0-9_-]{12,}/,
];
const liveTokenHits = [];
for (const file of tracked) {
  if (/\.(?:png|jpg|jpeg|ico|woff2|exe|pdf)$/i.test(file)) continue;
  let source;
  try { source = read(file); } catch { continue; }
  const suspicious = liveTokenPatterns.flatMap((pattern) => [...source.matchAll(new RegExp(pattern.source, 'g'))].map((match) => match[0]))
    .filter((value) => !/(?:test|replace|example)/i.test(value));
  if (suspicious.length) liveTokenHits.push(file);
}
assert('C8-NO-LIVE-TOKENS', liveTokenHits.length === 0, 'no tracked live-mode Stripe or webhook token shapes found', `possible live token material in: ${liveTokenHits.join(', ')}`);

const licensing = read('src-tauri/src/licensing.rs');
assert('C8-HOSTED-URL-BOUNDARY', licensing.includes('checkout.stripe.com') && licensing.includes('billing.stripe.com'),
  'desktop hosted billing URLs remain restricted to Stripe Checkout and Billing', 'hosted billing URL restrictions are missing');

const privacy = read('site/privacy.html');
const terms = read('site/terms.html');
const refunds = read('site/refunds.html');
assert('C8-POLICY-DRAFTS', [privacy, terms, refunds].every((value) => value.includes('Founder/legal review draft — not yet effective')),
  'all legal surfaces remain explicitly non-effective drafts', 'one or more legal surfaces could be mistaken for an effective policy');

const c6 = read('docs/engineering/C6-RELEASE-SAFETY.md');
assert('C8-DISTRIBUTION-HOLD', c6.includes('trusted public distribution explicitly deferred'),
  'unsigned internal installer is not represented as trusted public distribution', 'C6 public-distribution hold is missing');

const operationsRunbook = read('docs/engineering/FOUNDER-OPERATIONS-RUNBOOK.md');
assert('C8-OPERATIONS-RUNBOOK', [
  'Stripe sandbox',
  'SEV-1',
  'Webhook failed or delayed',
  'Security or privacy incident',
  'RELEASE_ROLLBACK_RUNBOOK.md',
  'Do not open public purchase or download',
].every((statement) => operationsRunbook.includes(statement)),
  'founder operations, incident, billing, and launch-hold procedures are present',
  'founder operations runbook is incomplete');

if (process.platform === 'win32' && existsSync(installer)) {
  try {
    const signature = execFileSync('pwsh.exe', ['-NoProfile', '-Command', `(Get-AuthenticodeSignature -LiteralPath '${installer.replaceAll("'", "''")}').Status`], { encoding: 'utf8', windowsHide: true }).trim();
    record('C8-SIGNATURE', 'deferred', `installer signature status is ${signature}; public release remains blocked by C6`);
  } catch {
    record('C8-SIGNATURE', 'deferred', 'Authenticode inspection is unavailable on this host; public release remains blocked by C6');
  }
} else {
  record('C8-SIGNATURE', 'deferred', 'Windows Authenticode inspection requires the Windows candidate host');
}

record('C8-CLEAN-PROFILE', 'deferred', 'interactive install, first run, upgrade, uninstall/reinstall, and 200% review require a disposable Windows profile');
record('C8-PAYMENT-LIFECYCLE', 'deferred', 'manual hosted lifecycle uses Stripe and Supabase sandboxes only; never live mode');
record('C8-LEGAL', 'deferred', 'founder/legal approval and publisher contact details are required before launch');

if (full) {
  const npmCli = resolve(dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js');
  const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
  run('C8-SITE', process.execPath, [npmCli, 'run', 'test:site']);
  run('C8-LINT', process.execPath, [npmCli, 'run', 'lint']);
  run('C8-FRONTEND-TESTS', process.execPath, [npmCli, 'test', '--', '--run']);
  run('C8-BUILD', process.execPath, [npmCli, 'run', 'build']);
  run('C8-RUST-FMT', cargo, ['fmt', '--all', '--', '--check'], resolve(root, 'src-tauri'));
  run('C8-RUST-TESTS', cargo, ['test', '--locked'], resolve(root, 'src-tauri'));
}

const failures = checks.filter((check) => check.status === 'fail');
console.log(`\nC8 preflight: ${checks.filter((check) => check.status === 'pass').length} passed, ${checks.filter((check) => check.status === 'deferred').length} deferred, ${failures.length} failed.`);
if (!full) console.log('Run `npm run audit:c8:full` for frontend and Rust quality gates.');
if (failures.length) process.exitCode = 1;
