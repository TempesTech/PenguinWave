# Packaging

Arch Linux assets for PenguinWave.

## Layout

```
packaging/
├── aur/
│   ├── penguinwave/       # source build (rust + npm) from a tagged release
│   └── penguinwave-bin/   # prebuilt: extracts the .deb from a GitHub Release
├── com.penguinwave.app.desktop   # desktop entry
├── 99-penguinwave.rules          # udev HID rule
└── icons/                        # hicolor theme icons (16–512 px + svg)
```

`penguinwave` and `penguinwave-bin` both `provides=penguinwave` and conflict,
so a user installs one or the other.

| Package | Build cost | Needs |
| --- | --- | --- |
| `penguinwave` | compiles Rust + frontend | rust, cargo, nodejs, npm |
| `penguinwave-bin` | none, unpacks `.deb` | a published Release asset |

## Build locally

```bash
cd aur/penguinwave      # or aur/penguinwave-bin
makepkg -si
```

Both pull from the `v$pkgver` tag, so cut the release tag first:
`git tag v0.1.0 && git push --tags`. `-bin` additionally needs the
`PenguinWave_$pkgver_amd64.deb` asset (Tauri's CamelCase productName, attach
it as-is) on that Release — produced by `tauri build`, uploaded by CI.

## Publish to AUR

```bash
makepkg --printsrcinfo > .SRCINFO     # run inside each aur/<pkg>/ dir
git clone ssh://aur@aur.archlinux.org/penguinwave.git aur-pkg
cp PKGBUILD .SRCINFO aur-pkg/
cd aur-pkg && git commit -am "penguinwave 0.1.0" && git push
```

(Repeat with the `penguinwave-bin` repo for the `-bin` package.)

## Release checklist

1. Bump `version` in `src-tauri/tauri.conf.json`, `Cargo.toml`, `package.json`.
2. Bump `pkgver` in both PKGBUILDs.
3. Tag + push (`v$pkgver`); CI builds and attaches the `.deb`.
4. `updpkgsums` in each PKGBUILD to replace the `SKIP` hashes.
5. Regenerate each `.SRCINFO`, push to the AUR repos.
