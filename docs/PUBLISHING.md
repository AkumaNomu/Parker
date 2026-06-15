# Publishing to GitHub

After extracting the repository, open a terminal in the `parker` directory.

## GitHub CLI

```powershell
git init
git add .
git commit -m "Initial Parker release"
gh repo create parker --public --source . --remote origin --push
```

Change `--public` to `--private` when needed.

## Existing empty repository

```powershell
git init
git add .
git commit -m "Initial Parker release"
git branch -M main
git remote add origin https://github.com/YOUR_USERNAME/parker.git
git push -u origin main
```

After the first push, CI builds Parker on a Windows runner. Push a tag matching
`Cargo.toml` to create setup EXE, portable EXE, ZIP, and SHA-256 assets:

```powershell
git tag v0.4.1
git push origin v0.4.1
```
