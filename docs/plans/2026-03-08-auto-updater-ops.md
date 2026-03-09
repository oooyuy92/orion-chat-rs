# Auto Updater Ops Notes

## Required GitHub Secrets

- `TAURI_SIGNING_PRIVATE_KEY`
  - The updater signing private key content.
  - Generated with `pnpm tauri signer generate`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  - The password for the encrypted updater signing private key.
  - Required when the generated minisign key is encrypted.

## Public Key

- The matching public key is committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
- Do not rotate the updater key casually; existing installed clients trust that public key.

## Release Checklist

1. Bump app version in:
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
   - `package.json`
2. Create and push tag like `v0.3.1`
3. Ensure GitHub Actions release workflow finishes successfully
4. Confirm release contains updater metadata and platform assets
5. Verify an older client can detect the new version
