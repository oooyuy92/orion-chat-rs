# Auto Updater Ops Notes

## Required GitHub Secrets

- `TAURI_SIGNING_PRIVATE_KEY`
  - The updater signing private key content.
  - Generated with `pnpm tauri signer generate`.
  - Keep this file only in local secure storage; do not commit it.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  - The password for the encrypted updater signing private key.
  - Required when the generated minisign key is encrypted.
  - Rotate this together with `TAURI_SIGNING_PRIVATE_KEY`.

## Public Key

- The matching public key is committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
- Do not rotate the updater key casually; existing installed clients trust that public key.
- The updater signing key was rotated on `2026-03-10`; any future rotation should update both secrets and the committed public key in a single change.

## Release Checklist

1. Bump app version in:
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
   - `package.json`
2. Create and push tag like `v0.3.2`
3. Ensure GitHub Actions release workflow finishes successfully
4. Confirm release contains updater metadata and platform assets
5. Verify an older client can detect the new version
