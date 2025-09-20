# Commit all changes
git add .
git commit -m "Initial release setup"
git tag v0.1.0
git push origin v0.1.0 


git tag -d v0.1.0
git push --delete origin v0.1.0
git tag v0.1.0
git push origin v0.1.0
