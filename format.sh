cargo fmt --all
git add .
git commit -m "cargo fmt --all"

cargo clippy --fix
git add .
git commit -m "cargo clippy --fix"
