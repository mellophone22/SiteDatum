import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { basename } from 'node:path';

export const RELEASE_MANIFEST_SCHEMA_VERSION = 1;

function validateCommit(value, label) {
  if (!/^[0-9a-f]{40}$/.test(value ?? '')) {
    throw new Error(`${label} must be a full lowercase Git commit SHA`);
  }
  return value;
}

function validateGeneratedAt(value) {
  if (!value || Number.isNaN(Date.parse(value)) || new Date(value).toISOString() !== value) {
    throw new Error('Generated timestamp must be an ISO 8601 UTC timestamp');
  }
  return value;
}

function validateSizeBytes(value) {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error('Installer size must be a positive safe integer');
  }
  return value;
}

function normalizeSha256(value) {
  const normalized = String(value ?? '').toUpperCase();
  if (!/^[0-9A-F]{64}$/.test(normalized)) {
    throw new Error('Installer SHA-256 must contain 64 hexadecimal characters');
  }
  return normalized;
}

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
    generatedAt: validateGeneratedAt(generatedAt),
    version,
    sourceCommit: validateCommit(sourceCommit, 'Source commit'),
    evidenceCommit: validateCommit(evidenceCommit, 'Evidence commit'),
    artifact: {
      filename: installerName,
      sizeBytes: validateSizeBytes(sizeBytes),
      sha256: normalizeSha256(sha256),
      authenticodeStatus: normalizeAuthenticodeStatus(authenticodeStatus),
    },
    publication: {
      date: publicationDate,
      downloadUrl,
    },
  };
}

export function assertManifestMatches({
  manifest,
  version,
  sourceCommit,
  evidenceCommit,
  installerName,
  sizeBytes,
  sha256,
  authenticodeStatus,
}) {
  if (manifest?.schemaVersion !== RELEASE_MANIFEST_SCHEMA_VERSION) {
    throw new Error(`Release manifest schema mismatch: expected ${RELEASE_MANIFEST_SCHEMA_VERSION}`);
  }
  if (manifest.channel !== 'unsigned-early-access') {
    throw new Error('Release manifest channel must be unsigned-early-access');
  }
  const publication = manifest.publication ?? {};
  const published = Boolean(publication.downloadUrl || publication.date);
  if (published && (!publication.downloadUrl || !publication.date)) {
    throw new Error('Release manifest publication metadata is incomplete');
  }
  const expectedState = published ? 'publication-candidate' : 'staged';
  if (manifest.state !== expectedState) {
    throw new Error(`Release manifest state mismatch: expected ${expectedState}`);
  }
  if (published) {
    validateDownloadUrl(publication.downloadUrl);
    validatePublicationDate(publication.date);
  }
  validateGeneratedAt(manifest.generatedAt);

  const expected = {
    version,
    sourceCommit: validateCommit(sourceCommit, 'Expected source commit'),
    evidenceCommit: validateCommit(evidenceCommit, 'Expected evidence commit'),
    filename: installerName,
    sizeBytes: validateSizeBytes(sizeBytes),
    sha256: normalizeSha256(sha256),
    authenticodeStatus: normalizeAuthenticodeStatus(authenticodeStatus),
  };
  const actual = {
    version: manifest.version,
    sourceCommit: manifest.sourceCommit,
    evidenceCommit: manifest.evidenceCommit,
    filename: manifest.artifact?.filename,
    sizeBytes: manifest.artifact?.sizeBytes,
    sha256: manifest.artifact?.sha256,
    authenticodeStatus: manifest.artifact?.authenticodeStatus,
  };
  for (const [key, expectedValue] of Object.entries(expected)) {
    if (actual[key] !== expectedValue) {
      throw new Error(`Release manifest ${key} mismatch`);
    }
  }
  return manifest;
}
