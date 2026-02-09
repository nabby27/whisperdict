# Release Process

This repository creates GitHub Releases automatically when a tag like `vX.Y.Z` is pushed.

## Steps (copy/paste)

1. Checkout `main` and pull the latest changes

```bash
git switch main
git pull --all --prune
```

2. Bump all required versions and commit

Update all files to the same version (example `X.Y.Z`):

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Then commit:

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "vX.Y.Z"
```

3. Create and push tag

```bash
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```
