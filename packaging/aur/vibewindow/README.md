# ShiJu AUR package

This directory contains the stable AUR package metadata for `paru -S shiju`.

## Before publishing

1. Create and push the matching GitHub release tag:

   ```bash
   git tag v0.2.3
   git push origin v0.2.3
   ```

2. On an Arch Linux machine, update the source checksum:

   ```bash
   cd packaging/aur/vibewindow
   updpkgsums
   makepkg --printsrcinfo > .SRCINFO
   makepkg -si
   ```

   Keep `sha256sums=('SKIP')` only for local smoke testing. The published AUR
   package should contain the real checksum produced by `updpkgsums`.

## Create the AUR package

1. Create an AUR account at https://aur.archlinux.org/register/.
2. Add your SSH public key in the AUR account settings.
3. Clone the empty AUR package repository:

   ```bash
   git clone ssh://aur@aur.archlinux.org/shiju.git
   ```

4. Copy the package files into that clone:

   ```bash
   cp packaging/aur/vibewindow/PKGBUILD shiju/
   cp packaging/aur/vibewindow/.SRCINFO shiju/
   ```

5. Commit and push:

   ```bash
   cd shiju
   git add PKGBUILD .SRCINFO
   git commit -m "Add shiju package"
   git push
   ```

After the push succeeds, users can install it with:

```bash
paru -S shiju
```

## Updating the package

When releasing a new version, update `pkgver`, reset `pkgrel` to `1`, run
`updpkgsums`, regenerate `.SRCINFO`, then commit and push to the AUR repository.
