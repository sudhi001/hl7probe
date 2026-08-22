# Releasing

## Cutting a release

1. Move the `Unreleased` entries in [`CHANGELOG.md`](CHANGELOG.md) under a new
   version heading, and add the comparison links at the bottom of that file.
   Update the sample `hl7probe --version` output in the README's Install
   section too: it is the one version number written down by hand.
2. Bump `version` in `Cargo.toml`, run `cargo test`, and commit.
3. Tag and push:

   ```sh
   git tag -a v0.1.0 -m "v0.1.0"
   git push origin v0.1.0
   ```

4. The `Release` workflow builds `hl7probe` for macOS (Apple silicon and Intel)
   and Linux (x86_64 and aarch64), then attaches the tarballs and their SHA-256
   sums to a GitHub release.

To rebuild an existing tag, run the workflow manually from the Actions tab and
pass the tag name.

## Refreshing the images in the README

The screenshots and the demo GIF are generated from real output, so they cannot
drift from what the tool prints:

```sh
python3 docs/tools/render_media.py
```

That rebuilds `docs/report.svg`, `docs/tui.svg` and `docs/demo.gif`. It needs
Pillow for the GIF (`pip install pillow`) and a font at
`/System/Library/Fonts/Menlo.ttc`, so it is a macOS convenience script rather
than part of the build.

## Updating the Homebrew formula

The formula lives in [`sudhi001/homebrew-tap`](https://github.com/sudhi001/homebrew-tap),
not here: it records the checksum of a tag of this repository, so it cannot sit
inside the tag it describes.

It builds from the tagged source tarball, so a release only needs the new URL
and checksum:

```sh
version=0.1.0
url="https://github.com/sudhi001/hl7probe/archive/refs/tags/v${version}.tar.gz"
curl -fsSL -o "hl7probe-${version}.tar.gz" "$url"
tar -tzf "hl7probe-${version}.tar.gz" >/dev/null   # reject error pages
shasum -a 256 "hl7probe-${version}.tar.gz"
```

Put the `url` and `sha256` into `Formula/hl7probe.rb` in the tap, then verify:

```sh
brew style Formula/hl7probe.rb
brew install --build-from-source sudhi001/tap/hl7probe
brew test hl7probe
```

Users install with:

```sh
brew install sudhi001/tap/hl7probe
```

The bare `brew install hl7probe` needs the formula to live in homebrew-core,
which accepts a project once it is established; until then the tap prefix, or a
one-off `brew tap sudhi001/tap`, is required.
