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

After the first push, the CI workflow builds Parker on a Windows runner. Push a
tag to create a release ZIP:

```powershell
git tag v0.3.0
git push origin v0.3.0
```
