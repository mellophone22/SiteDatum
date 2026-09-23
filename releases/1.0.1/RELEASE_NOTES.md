# AnyDesk 1.0.1

This maintenance release fixes cloud workspace authentication persistence on Windows.

## Fixed

- Enabled keyring's native Windows Credential Manager backend. Version 1.0.0 unintentionally used keyring's nonpersistent mock backend, so the UI could show **Connected** immediately after sign-in while **Sync now** could not retrieve the session.
- Added a regression test that requires the persistent Windows credential backend.
- Improved cloud-session errors to distinguish a missing sign-in from an unavailable credential store.

## Installation and recovery

Install `AnyDesk_1.0.1_x64-setup.exe` on every computer that uses cloud synchronization. After upgrading, open **Settings**, disconnect the previous displayed session if necessary, then sign in again on each computer. The new session is stored in Windows Credential Manager and survives application restarts.

Local projects, files, and SQLite workspace data are not removed by the upgrade or by disconnecting the broken session.

The binaries are not code-signed. Windows SmartScreen may display an unrecognized-publisher warning.
