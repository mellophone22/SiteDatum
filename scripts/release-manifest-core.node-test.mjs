import assert from 'node:assert/strict';
import test from 'node:test';
import {
  assertManifestMatches,
  assertExpectedSignature,
  assertInstallerName,
  assertSynchronizedVersions,
  createManifest,
  normalizeAuthenticodeStatus,
  parseCargoVersion,
  validateDownloadUrl,
  validatePublicationDate,
} from './release-manifest-core.mjs';

test('parses and synchronizes release versions', () => {
  assert.equal(parseCargoVersion('[package]\nversion = "1.4.1"\n'), '1.4.1');
  assert.equal(assertSynchronizedVersions({ packageVersion: '1.4.1', tauriVersion: '1.4.1', cargoVersion: '1.4.1' }), '1.4.1');
  assert.throws(() => assertSynchronizedVersions({ packageVersion: '1.4.1', tauriVersion: '1.4.0', cargoVersion: '1.4.1' }), /mismatch/);
});

test('requires the versioned NSIS filename', () => {
  assert.equal(assertInstallerName('C:\\release\\SiteDatum_1.4.1_x64-setup.exe', '1.4.1'), 'SiteDatum_1.4.1_x64-setup.exe');
  assert.throws(() => assertInstallerName('SiteDatum_latest.exe', '1.4.1'), /filename mismatch/);
});

test('validates expected Authenticode state', () => {
  assert.equal(normalizeAuthenticodeStatus('NotSigned\r\n'), 'NotSigned');
  assert.doesNotThrow(() => assertExpectedSignature('NotSigned', 'NotSigned'));
  assert.throws(() => assertExpectedSignature('Valid', 'NotSigned'), /status mismatch/);
  assert.throws(() => normalizeAuthenticodeStatus('SurprisingState'), /Unrecognized/);
});

test('restricts publication metadata to approved HTTPS origins', () => {
  assert.equal(validateDownloadUrl(null), null);
  assert.equal(validateDownloadUrl('https://sitedatum.site/downloads/SiteDatum.exe'), 'https://sitedatum.site/downloads/SiteDatum.exe');
  assert.equal(validateDownloadUrl('https://github.com/mellophone22/SiteDatum/releases/download/v1.4.1/app.exe'), 'https://github.com/mellophone22/SiteDatum/releases/download/v1.4.1/app.exe');
  assert.throws(() => validateDownloadUrl('https://github.com/another-owner/SiteDatum/releases/download/v1.4.1/app.exe'), /canonical SiteDatum GitHub release/);
  assert.throws(() => validateDownloadUrl('http://sitedatum.site/app.exe'), /HTTPS/);
  assert.throws(() => validateDownloadUrl('https://example.com/app.exe'), /HTTPS/);
  assert.equal(validatePublicationDate('2026-10-03'), '2026-10-03');
  assert.throws(() => validatePublicationDate('10/03/2026'), /YYYY-MM-DD/);
});

test('keeps staged and publication-candidate manifests distinct', () => {
  const common = {
    generatedAt: '2026-10-03T12:00:00.000Z',
    version: '1.4.1',
    sourceCommit: 'a'.repeat(40),
    evidenceCommit: 'b'.repeat(40),
    installerName: 'SiteDatum_1.4.1_x64-setup.exe',
    sizeBytes: 4681099,
    sha256: 'C'.repeat(64),
    authenticodeStatus: 'NotSigned',
  };
  assert.equal(createManifest(common).state, 'staged');
  assert.equal(createManifest({ ...common, downloadUrl: 'https://sitedatum.site/app.exe', publicationDate: '2026-10-03' }).state, 'publication-candidate');
  assert.throws(() => createManifest({ ...common, downloadUrl: 'https://sitedatum.site/app.exe' }), /supplied together/);
});

test('verifies provenance against independently observed artifact evidence', () => {
  const evidence = {
    generatedAt: '2026-10-08T18:00:00.000Z',
    version: '1.4.3',
    sourceCommit: 'a'.repeat(40),
    evidenceCommit: 'b'.repeat(40),
    installerName: 'SiteDatum_1.4.3_x64-setup.exe',
    sizeBytes: 5_000_000,
    sha256: 'c'.repeat(64),
    authenticodeStatus: 'NotSigned',
  };
  const manifest = createManifest(evidence);
  assert.equal(assertManifestMatches({ ...evidence, manifest }), manifest);
  assert.throws(
    () => assertManifestMatches({ ...evidence, manifest, sha256: 'd'.repeat(64) }),
    /sha256 mismatch/,
  );
  assert.throws(
    () => assertManifestMatches({ ...evidence, manifest: { ...manifest, state: 'publication-candidate' } }),
    /state mismatch/,
  );
  assert.throws(
    () => createManifest({ ...evidence, sourceCommit: 'short' }),
    /full lowercase Git commit SHA/,
  );
});
