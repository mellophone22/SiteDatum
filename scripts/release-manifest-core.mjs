import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { basename } from 'node:path';

export const RELEASE_MANIFEST_SCHEMA_VERSION = 1;

export function parseCargoVersion(source) {
  return source.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
}

export function assertSynchronizedVersions({ packageVersion, tauriVersion, cargoVersion }) {
  const versions = [packageVersion, tauriVersion, cargoVersion];
  if (versions.some((version) => !version) || new Set(versions).size !== 1) {
    throw new Error(
      `Release version mismatch: package=${packageVersion ?? 'missing'}, ` +
      `tauri=${tauriVersion ?? 'missing'}, cargo=${cargoVersion ?? 'missing'}`,
    );
  }
  return packageVersion;
}

export function assertInstallerName(path, version) {
  const expected = `SiteDatum_${version}_x64-setup.exe`;
  const actual = basename(path);
  if (actual !== expected) {
    throw new Error(`Installer filename mismatch: expected ${expected}, received ${actual}`);
  }
  return expected;
}

export function normalizeAuthenticodeStatus(status) {
  const normalized = String(status).trim();
  const known = new Set(['Valid', 'NotSigned', 'UnknownError', 'HashMismatch', 'NotTrusted']);
  if (!known.has(normalized)) {
    throw new Error(`Unrecognized Authenticode status: ${normalized || 'empty'}`);
  }
  return normalized;
}

export function assertExpectedSignature(actual, expected) {
  if (actual !== expected) {
    throw new Error(`Authenticode status mismatch: expected ${expected}, received ${actual}`);
  }
}

export function validateDownloadUrl(value) {
  if (!value) return null;
  const url = new URL(value);
  const canonicalSite = url.hostname === 'sitedatum.site' || url.hostname === 'www.sitedatum.site';
  const canonicalGitHubRelease = url.hostname === 'github.com'
    && /^\/mellophone22\/SiteDatum\/releases\/download\//.test(url.pathname);
  if (url.protocol !== 'https:' || (!canonicalSite && !canonicalGitHubRelease) || url.username || url.password || url.hash) {
    throw new Error('Download URL must be an HTTPS SiteDatum URL or canonical SiteDatum GitHub release URL without credentials or a fragment');
  }
  return url.toString();
}

export function validatePublicationDate(value) {
  if (!value) return null;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value) || Number.isNaN(Date.parse(`${value}T00:00:00Z`))) {
    throw new Error('Publication date must use YYYY-MM-DD');
  }
  return value;
}

export async function sha256File(path) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest('hex').toUpperCase();
}

export function createManifest({
  generatedAt,
  version,
  sourceCommit,
  evidenceCommit,
  installerName,
  sizeBytes,
  sha256,
  authenticodeStatus,
  downloadUrl = null,
  publicationDate = null,
}) {
  const published = Boolean(downloadUrl || publicationDate);
  if (published && (!downloadUrl || !publicationDate)) {
    throw new Error('Publication date and download URL must be supplied together');
  }

  return {
    schemaVersion: RELEASE_MANIFEST_SCHEMA_VERSION,
    channel: 'unsigned-early-access',
    state: published ? 'publication-candidate' : 'staged',
    generatedAt,
    version,
    sourceCommit,
    evidenceCommit,
    artifact: {
      filename: installerName,
      sizeBytes,
      sha256,
      authenticodeStatus,
    },
    publication: {
      date: publicationDate,
      downloadUrl,
    },
  };
}
