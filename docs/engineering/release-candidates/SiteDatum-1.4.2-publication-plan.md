# SiteDatum 1.4.2 publication plan

**State:** Prepared but blocked from publication

## Immutable candidate identity

- Source commit: `8a5e25f1c8bac3de861ae16a88a0ee8389036cc3`
- Installer: `SiteDatum_1.4.2_x64-setup.exe`
- Size: 4,681,448 bytes
- SHA-256: `86CB0CFDC1106F022745875C6DC8DA6BB52C891D0B4C445C9FC6F1AE289EEFA3`
- Authenticode: `NotSigned`
- Intended canonical URL: `https://sitedatum.site/downloads/SiteDatum_1.4.2_x64-setup.exe`

The local staged installer must remain byte-for-byte identical to this identity. If any source, configuration, packaging input, or executable byte changes, discard this publication plan and generate a new candidate, hash, and manifest.

## Focused acceptance still required

- Install 1.4.2 in the established disposable Windows profile and confirm first-run startup.
- Upgrade an existing 1.4.1 fictional workspace and confirm tasks, RFIs, submittals, files, notes, contacts, and ordinary project documents remain available.
- Confirm production account sign-in and entitlement refresh without completing Checkout.
- Confirm offline local work, backup, essential CSV export, and safe Free fallback remain available.
- Uninstall and reinstall the exact candidate and confirm the fictional workspace reopens.
- Complete a focused keyboard and 200% scaling smoke review.

## Prepared website publication values

After focused acceptance and the separately authorized live lifecycle pass, update every website download label and release identity from 1.4.1 to 1.4.2, replace the installer filename, size, source commit, SHA-256, publication date, and PowerShell verification command with the immutable values above, and add concise 1.4.2 release notes. Run `npm run test:site` before deployment.

Do not upload the installer, deploy these website values, create a public release tag, enable purchase, or describe 1.4.2 as paid-production-ready before both remaining gates pass.
