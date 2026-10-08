import { execFileSync } from 'node:child_process';
import { readFileSync, statSync } from 'node:fs';
import { basename, isAbsolute, relative, resolve, sep } from 'node:path';
import process from 'node:process';
import {
  assertExpectedSignature,
  assertManifestMatches,
  assertSynchronizedVersions,
  normalizeAuthenticodeStatus,
  parseCargoVersion,
  sha256File,
} from './release-manifest-core.mjs';

const root = resolve(import.meta.dirname, '..');

function fail(message) {
  console.error(`Release manifest verification failed: ${message}`);
  process.exit(1);
}

function option(name) {
  const index = process.argv.indexOf(name);
  if (index === -1) return null;
  const value = process.argv[index + 1];
  if (!value || value.startsWith('--')) fail(`${name} requires a value`);
  return value;
}

function requiredOption(name) {
  const value = option(name);
  if (!value) fail(`${name} is required`);
  return value;
}

function repositoryPath(value, label) {
  const path = resolve(root, value);
  const rel = relative(root, path);
  if (rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel)) fail(`${label} must stay inside the repository`);
  return path;
}

function git(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', windowsHide: true }).trim();
}

function readGitFile(commit, path) {
  return git(['show', `${commit}:${path}`]);
}

function authenticodeStatus(installer) {
  if (process.platform !== 'win32') fail('Authenticode verification requires Windows');
  const quotedInstaller = `'${installer.replaceAll("'", "''")}'`;
  const script = `(Get-AuthenticodeSignature -LiteralPath ${quotedInstaller}).Status.ToString()`;
  for (const shell of ['pwsh.exe', 'powershell.exe']) {
    try {
      return normalizeAuthenticodeStatus(execFileSync(shell, ['-NoProfile', '-NonInteractive', '-Command', script], {
        encoding: 'utf8',
        windowsHide: true,
      }));
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
  fail('PowerShell is required for Authenticode verification');
}

try {
  const manifestPath = repositoryPath(requiredOption('--manifest'), 'Manifest path');
  const installer = repositoryPath(requiredOption('--installer'), 'Installer path');
  const sourceCommit = git(['rev-parse', `${requiredOption('--source-commit')}^{commit}`]);
  const evidenceCommit = git(['rev-parse', `${requiredOption('--evidence-commit')}^{commit}`]);
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const expectedSignature = option('--expect-signature') ?? 'NotSigned';
  const signature = authenticodeStatus(installer);
  assertExpectedSignature(signature, expectedSignature);

  const packageJson = JSON.parse(readGitFile(sourceCommit, 'package.json'));
  const tauriConfig = JSON.parse(readGitFile(sourceCommit, 'src-tauri/tauri.conf.json'));
  const cargoVersion = parseCargoVersion(readGitFile(sourceCommit, 'src-tauri/Cargo.toml'));
  const version = assertSynchronizedVersions({
    packageVersion: packageJson.version,
    tauriVersion: tauriConfig.version,
    cargoVersion,
  });

  assertManifestMatches({
    manifest,
    version,
    sourceCommit,
    evidenceCommit,
    installerName: basename(installer),
    sizeBytes: statSync(installer).size,
    sha256: await sha256File(installer),
    authenticodeStatus: signature,
  });

  console.log('Release manifest verification passed.');
  console.log(`Version: ${version}`);
  console.log(`Source commit: ${sourceCommit}`);
  console.log(`Evidence commit: ${evidenceCommit}`);
  console.log(`SHA-256: ${manifest.artifact.sha256}`);
  console.log(`State: ${manifest.state}`);
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
