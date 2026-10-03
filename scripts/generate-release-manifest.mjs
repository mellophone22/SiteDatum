import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import process from 'node:process';
import {
  assertExpectedSignature,
  assertInstallerName,
  assertSynchronizedVersions,
  createManifest,
  normalizeAuthenticodeStatus,
  parseCargoVersion,
  sha256File,
  validateDownloadUrl,
  validatePublicationDate,
} from './release-manifest-core.mjs';

const root = resolve(import.meta.dirname, '..');

function fail(message) {
  console.error(`Release manifest failed: ${message}`);
  process.exit(1);
}

function git(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', windowsHide: true }).trim();
}

function readGitFile(commit, path) {
  return git(['show', `${commit}:${path}`]);
}

function option(name) {
  const index = process.argv.indexOf(name);
  if (index === -1) return null;
  const value = process.argv[index + 1];
  if (!value || value.startsWith('--')) fail(`${name} requires a value`);
  return value;
}

function repositoryPath(value, label) {
  const path = resolve(root, value);
  const rel = relative(root, path);
  if (rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel)) fail(`${label} must stay inside the repository`);
  return path;
}

function authenticodeStatus(installer) {
  if (process.platform !== 'win32') fail('Authenticode inspection requires Windows');
  const script = '$signature = Get-AuthenticodeSignature -LiteralPath $args[0]; $signature.Status.ToString()';
  const shells = ['pwsh.exe', 'powershell.exe'];
  for (const shell of shells) {
    try {
      return normalizeAuthenticodeStatus(execFileSync(shell, ['-NoProfile', '-NonInteractive', '-Command', script, installer], {
        encoding: 'utf8',
        windowsHide: true,
      }));
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
  fail('PowerShell is required for Authenticode inspection');
}

try {
  const dirty = git(['status', '--porcelain=v1', '--untracked-files=normal']);
  if (dirty) fail(`repository is not clean:\n${dirty}`);

  const sourceCommit = git(['rev-parse', `${option('--source-commit') ?? 'HEAD'}^{commit}`]);
  const evidenceCommit = git(['rev-parse', 'HEAD^{commit}']);
  const packageJson = JSON.parse(readGitFile(sourceCommit, 'package.json'));
  const tauriConfig = JSON.parse(readGitFile(sourceCommit, 'src-tauri/tauri.conf.json'));
  const cargoVersion = parseCargoVersion(readGitFile(sourceCommit, 'src-tauri/Cargo.toml'));
  const version = assertSynchronizedVersions({
    packageVersion: packageJson.version,
    tauriVersion: tauriConfig.version,
    cargoVersion,
  });

  const defaultInstaller = `src-tauri/target/release/bundle/nsis/SiteDatum_${version}_x64-setup.exe`;
  const installer = repositoryPath(option('--installer') ?? defaultInstaller, 'Installer path');
  if (!existsSync(installer)) fail(`installer does not exist: ${installer}`);
  const installerName = assertInstallerName(installer, version);

  const expectedSignature = option('--expect-signature') ?? 'NotSigned';
  const signature = authenticodeStatus(installer);
  assertExpectedSignature(signature, expectedSignature);

  const downloadUrl = validateDownloadUrl(option('--download-url'));
  const publicationDate = validatePublicationDate(option('--publication-date'));
  const manifest = createManifest({
    generatedAt: new Date().toISOString(),
    version,
    sourceCommit,
    evidenceCommit,
    installerName,
    sizeBytes: statSync(installer).size,
    sha256: await sha256File(installer),
    authenticodeStatus: signature,
    downloadUrl,
    publicationDate,
  });

  const defaultOutput = `tmp/release-manifest-${version}.json`;
  const output = repositoryPath(option('--output') ?? defaultOutput, 'Output path');
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' });

  console.log(`Release manifest written: ${output}`);
  console.log(`Version: ${version}`);
  console.log(`Source commit: ${sourceCommit}`);
  console.log(`SHA-256: ${manifest.artifact.sha256}`);
  console.log(`Authenticode: ${signature}`);
  console.log(`State: ${manifest.state}`);
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
