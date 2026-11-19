# Packaging and release artifacts

The release workflow builds four downloadable artifacts:

| Platform | Artifact | Notes |
| --- | --- | --- |
| Windows x64 | NSIS installer (`.exe`) | Current-user installer; Windows x64 runner is checked before building. |
| Windows x64 | Portable ZIP | Contains `EasyMD.exe`, `LICENSE`, and `README-portable.txt`; no installer is run. |
| macOS Apple Silicon | DMG (`aarch64.dmg`) | Built on an Apple Silicon runner and checked for an `arm64` Mach-O executable. |
| macOS Intel | DMG (`x64.dmg`) | Built on the `macos-15-intel` runner and checked for an `x86_64` Mach-O executable. |

The Intel runner label is listed in the [GitHub-hosted runners reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Release builds verify the runner's reported architecture and the main executable architecture, so a mislabeled or wrong-architecture build fails instead of being uploaded under the wrong platform name.

## Windows portable ZIP

The portable ZIP is an extracted-folder distribution, not an installer or a self-contained runtime. Extract it to a writable directory and run `EasyMD.exe`. It includes the repository's `LICENSE` beside the executable and a bilingual `README-portable.txt` with requirements and data-location notes.

The app requires Windows x64 and the Microsoft Edge WebView2 Runtime. Windows 11 includes WebView2; Windows 10 may need the Evergreen WebView2 Runtime installed separately.

Portable does **not** mean that application data follows the executable. EasyMD settings, indexes, and other private configuration remain under the current user's AppData directory. Markdown vaults remain wherever the user chose to keep them; the portable archive does not relocate or copy vault data.

The portable packaging script validates the executable's PE machine type as AMD64, checks configured Tauri `externalBin`/resource settings, then verifies that the completed ZIP contains exactly the EXE, license, and README. It also checks that the archived `LICENSE` is byte-for-byte identical to the repository license. New sidecars or extra Tauri resources intentionally fail packaging until included explicitly. Current runtime process calls invoke platform facilities or a user-selected command; there are no configured Tauri `externalBin` sidecars.

## macOS signing

The workflow retains the existing ad-hoc re-signing step after copying the `.app` bundle, verifies the signature, and creates/verifies the final DMG. Ad-hoc signing is not Developer ID signing or notarization; Gatekeeper may still warn when opening the downloaded app.

## Running and publishing

`Release builds` runs for a `v*` tag push or by manual dispatch and uploads build artifacts only; it does not create a GitHub Release. The existing `Publish verified release` workflow is a one-off workflow pinned to the already-published `v0.4.6` run. Do not reuse it for these builds: a future release must use a new version/tag and update its source-run and expected-file checks to cover all four packages. Do not overwrite the existing `v0.4.6` release as a test target.

For a local Windows build, run these commands from the repository root in PowerShell:

```powershell
npm ci
npm run tauri build -- --ci
$version = node -p "require('./package.json').version"
./scripts/package-windows-portable.ps1 -BinaryPath 'src-tauri/target/release/EasyMD.exe' -LicensePath 'LICENSE' -OutputDirectory 'artifacts/portable' -Version $version
```

If you override `CARGO_TARGET_DIR`, adjust `-BinaryPath` accordingly. The script refuses to overwrite an existing ZIP.

For macOS, build on the matching architecture with the platform overlay:

```bash
npm ci
npm run tauri build -- --config src-tauri/tauri.macos.conf.json --ci
```

The CI matrix uses native Apple Silicon and Intel hosts. Mac `.app` bundles include the license in `Contents/Resources/LICENSE`; Windows installers and Portable ZIPs include it beside the executable. About also embeds the same full license text.
