# Distribution

Editio is MIT licensed, copyright Nicolas Germineau. The first version is 0.1
(`0.1.0` in Cargo, tags and package metadata). The framework stays private in
`../tapp-ui`. Users need only a binary, not Rust or the framework. Public source
checkouts cannot build without access to that framework.

No CI, automatic build hooks, registry submissions, or additional readiness audit.
Issues and PRs are welcome; accepting a PR does not grant repository access.

## Install and update

- **mise (including Omarchy/Arch):** `mise use -g github:gerukin/editio@latest`.
  Update with `mise upgrade github:gerukin/editio`. mise's default 24-hour release
  age rule can temporarily hide a brand-new version from `@latest`. Install
  `github:gerukin/editio@0.1.2` explicitly during that window; use
  `mise use -g github:gerukin/editio@latest` afterward to follow releases.
- **Linux/macOS installer:** use the one-liner below for installation and updates.
  Append `-s -- 0.1.2` to `sh` to select a specific version.
  Default: `~/.local/bin`; override with `EDITIO_INSTALL_DIR`.
- **Homebrew:** `brew tap gerukin/editio https://github.com/gerukin/editio`, then
  `brew install --cask gerukin/editio/editio` on macOS. Update with
  `brew upgrade --cask editio`. This binary cask avoids unnecessary Xcode build
  requirements; bundled notices remain in Homebrew's Caskroom. On Linux, use
  `brew install --formula gerukin/editio/editio` and `brew upgrade editio`.
  Both recipes live in this repository (`Casks/` and `Formula/`); no second
  repository or Homebrew core submission is needed.
- **Windows/manual:** extract the archive for your OS/architecture from GitHub
  Releases and add its directory to PATH. To update, close Editio and replace
  the executable. Keep the included licenses with the distribution.

```sh
curl -fsSL https://github.com/gerukin/editio/releases/latest/download/install.sh | sh
```

The release workflow uploads `install.sh`; this URL selects the latest published release.

No WinGet, AUR, Scoop, pixi, npm or crates.io publication. No self-updater.
Installation never changes EDITOR/VISUAL, terminal configuration or file
associations. Omarchy default-editor registration remains a separate future task.

## Six release targets

| Target | Status |
| --- | --- |
| Linux x86-64 | Tested on the developer's machine; not all Linux distributions |
| Linux ARM64 | Untested |
| macOS x86-64 | Untested |
| macOS ARM64 | Tested on two developer-owned Macs |
| Windows x86-64 | Tested on the developer’s Surface |
| Windows ARM64 | Untested |

These are 64-bit targets, not 32-bit x86/ARM. Planned baselines: Linux glibc 2.28+,
macOS 11+. Linux musl/Alpine is not covered by the GNU archive. Cross-compilation
success does not imply runtime testing. All six targets are cross-compiled locally. Only the machines listed above have
been exercised; this is not a claim of coverage of every OS or terminal version.

## Unsigned downloads

macOS executables have an ad-hoc linker signature, but no Apple Developer ID
signature or notarization. Windows executables are unsigned. Gatekeeper,
SmartScreen or application-control policies can warn or refuse execution.
Browser quarantine behavior has not been comprehensively tested. The installer
does not disable security settings or remove quarantine. Signing/notarization
is deferred to a later release. Windows bundles use a static C runtime and do
not require a separate Visual C++ runtime installation.

## Local development

`cargo run -- notes.md` / `cargo build` build only for this machine.
The `~/.local/bin/editio` symlink points to `target/release/editio`. To refresh
the native optimized binary, verify its version and update the symlink, run
`cargo run --locked --manifest-path tools/release/Cargo.toml -- refresh-local`.
Nothing implicitly builds other platforms.

## Local publishing

The separate Rust helper has no dependencies on the app or framework:

```sh
cargo run --locked --manifest-path tools/release/Cargo.toml -- plan
```

Before first publication:

1. Local Git tracking is initialized. Create `gerukin/editio` on GitHub with
   `origin` pointing to it. `.local`, agent
   configuration, secrets and build outputs are ignored. Do not copy tapp-ui.
2. Install the six Rust target libraries listed by `plan`, Zig, cargo-zigbuild,
   cargo-xwin, cargo-about (`cargo install --locked --features cli cargo-about`),
   LLVM/Clang, tar, zip, sha256sum and gh. Publishing runs on this Linux machine.
   Set `SDKROOT` to a legally obtained macOS SDK; Windows builds require the
   Microsoft SDK/toolchain terms. Normal builds do not download SDKs.
3. Homebrew uses this same repository as a custom tap. The helper generates
   `Formula/editio.rb` and `Casks/editio.rb` content using actual archive checksums.

For each release, update Cargo's version and move the relevant `Unreleased`
entries under `## [<version>] - YYYY-MM-DD` (using the actual version/date). Keep an
empty `Unreleased` section above it, following `gerukin/ai-tester`. Commit Editio
and framework changes. Then explicitly run:

```sh
cargo run --locked --manifest-path tools/release/Cargo.toml -- publish
```

This builds all six optimized binaries into `target/publish` (Unix) and `target/publish-windows-static` (Windows), bundles licenses,
creates archives/checksums and Homebrew recipes under `dist/<version>`, pushes
Editio's HEAD to `origin/main`, and uploads a **draft** GitHub Release. Nothing is
pushed until all six builds/package steps succeed. Framework source is never
uploaded. Both source commit IDs are recorded in the archives. The helper checks
the native binary's `--version` before and after archiving.

Linux uses Zig with a glibc 2.28 baseline. macOS uses Clang and Rust's bundled
`ld64.lld` with the local SDK to enforce macOS 11 (Zig may raise that minimum).
Windows uses cargo-xwin with the static C runtime.

Finalize the draft using the printed command. Copy the generated `editio.rb` to
`Formula/editio.rb` and `editio.cask.rb` to `Casks/editio.rb` in this repository,
commit, and push them. This is only
package metadata: no rebuild. Existing releases are not overwritten. If upload
fails, recover the draft using `gh release upload` and the existing `dist` files;
do not rebuild unless sources changed.

After publication, run
`cargo run --locked --manifest-path tools/release/Cargo.toml -- refresh-local`
and confirm `editio --version` matches the released version. This refreshes only
the local development installation.

The shell installer and Homebrew recipes are small platform packaging adapters;
the editor and local publishing tool remain Rust.

## 0.1.0 installation checks

The public curl install/reinstall route was exercised on Linux x86-64 and macOS
ARM64. Explicit `mise ...@0.1.0` installation passed on Linux; `@latest` remains
subject to mise's 24-hour release-age rule. Homebrew cask installation and its
already-current upgrade check passed on macOS, but the quarantined first launch
stalled over SSH. No security settings were changed; notarization is deferred.
The same Mac ran the curl-installed executable successfully.

Windows x86-64 was tested on Surface before publication; a fresh public ZIP test
could not run because that machine was unreachable. The remaining architectures
are cross-compiled and untested. A repeat install/update check is not evidence of
upgrading between two different release versions (0.1.0 is the first release).

## 0.1.1 installation checks

All six uploaded archives were downloaded and verified against SHA256SUMS.
Both Linux builds require at most glibc 2.28; both macOS builds declare minimum
macOS 11. Both Windows builds import system DLLs without a separate VC runtime.
The public installer upgraded an isolated Linux installation from 0.1.0 to
0.1.1 successfully. Pinned mise 0.1.1 installation also passed. Homebrew recipes
passed Ruby syntax checks; a live Homebrew upgrade was not repeated this release.
User testing of the feature update passed on Linux x86-64, the personal ARM64 Mac
and x86-64 Surface before publication. Linux ARM64, macOS Intel and Windows ARM64
remain untested at runtime. Developer installations were left unchanged.

## 0.1.2 installation checks

All six uploaded archives were downloaded and verified against SHA256SUMS.
Both Linux builds require at most glibc 2.28; both macOS builds declare minimum
macOS 11. Both Windows builds import system DLLs without a separate VC runtime.
The generated Homebrew recipes passed Ruby syntax checks. The local Linux
development executable was rebuilt and reports 0.1.2.

The public installer upgraded the personal ARM64 Mac's active installation from
0.1.0 to 0.1.2. Homebrew upgraded the work ARM64 Mac from 0.1.1 to 0.1.2;
after the user approved execution through macOS, its version check passed.
Both installed Mac executables match the released binary's SHA256. These checks
verify installation and startup, not native terminal selection gestures.
