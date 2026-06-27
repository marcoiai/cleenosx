# Mac App Store Preparation

cleenosx can be packaged for the Mac App Store as a MealWare product, but the Store build must be more restricted than the power-user/dev build.

## Current Bundle Identity

- Product name: `cleenosx`
- Company brand: `MealWare`
- Bundle identifier: `dev.cleanerx.desktop`
- Category: `Utility`
- Minimum macOS: `12.0`

The visible app name has been rebranded to cleenosx under the MealWare company brand. The bundle identifier remains `dev.cleanerx.desktop` until the internal identifier refactor is done intentionally.

The Bundle ID created in Apple Developer and App Store Connect must exactly match `dev.cleanerx.desktop`, or the identifier must be changed in `src-tauri/tauri.conf.json` before creating Apple profiles.

## Store Build Behavior

Builds compiled with `CLEANERX_DISTRIBUTION=app-store` intentionally disable:

- Recovery helper UI.
- Recovery script export.
- Administrator cleanup continuation through `osascript`.
- Full Disk Access settings shortcut.

The normal/dev build keeps those power-user flows. This split exists because Mac App Store apps must run inside App Sandbox and should not ship privileged cleanup helpers.

## Required Apple Account Work

1. Enroll in Apple Developer Program.
2. Create an App ID with Bundle ID `dev.cleanerx.desktop`.
3. Create/download a `Mac App Store Connect` provisioning profile for that App ID.
4. Put the profile at:

```text
src-tauri/profiles/cleenosx_Mac_App_Store.provisionprofile
```

5. Edit `src-tauri/Entitlements.appstore.plist` and replace both `TEAM_ID` placeholders with the Apple Team ID / App ID prefix.
6. Create App Store Connect app record with the same Bundle ID.
7. Create an App Store Connect API key for upload.

## Local Store Build

```sh
pnpm tauri:appstore
```

This runs:

```sh
CLEANERX_DISTRIBUTION=app-store VITE_CLEANERX_DISTRIBUTION=app-store tauri build --no-bundle
CLEANERX_DISTRIBUTION=app-store VITE_CLEANERX_DISTRIBUTION=app-store tauri bundle --bundles app --config src-tauri/tauri.appstore.conf.json
```

For universal Intel + Apple Silicon builds, add the required Rust targets and use Tauri's universal macOS target once signing is configured.

## Create PKG For Upload

After a signed `.app` exists, create the upload package with a Mac Installer Distribution certificate:

```sh
xcrun productbuild \
  --sign "3rd Party Mac Developer Installer: YOUR_NAME (TEAM_ID)" \
  --component "target/release/bundle/macos/cleenosx.app" \
  /Applications \
  cleenosx.pkg
```

Then upload with App Store Connect API credentials:

```sh
xcrun altool \
  --upload-app \
  --type macos \
  --file cleenosx.pkg \
  --apiKey "$APPLE_API_KEY_ID" \
  --apiIssuer "$APPLE_API_ISSUER"
```

## Product Review Risks

- Disk cleaners are inherently sensitive. The Store copy should be framed as an inspector and safe cleanup assistant, not a system bypass tool.
- App Sandbox may limit arbitrary filesystem scanning. Expect to add user-selected folder access, scoped bookmarks, or a simpler "Choose Folder to Scan" flow before final review.
- Deletion must stay transparent: selected IDs only, backend allowlist, two-step confirmation, and clear failure reporting.
- Recovery and root workflows should remain outside the Store build.

## References

- Tauri App Store guide: https://v2.tauri.app/distribute/app-store/
- Tauri macOS signing guide: https://v2.tauri.app/distribute/sign/macos/
- Apple App Sandbox entitlement: https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.app-sandbox
