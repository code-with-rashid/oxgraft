# packaging/

Distribution wrappers so people can install Oxgraft from their usual package
manager. The Rust crate `oxgraft` (in `crates/oxgraft-cli`) is the real product.

| Channel | Folder | Today | Later |
|---|---|---|---|
| crates.io | `crates/oxgraft-cli` | 0.0.1 reserves the name | `cargo install oxgraft` |
| npm | `npm/` | 0.0.1 prints install guidance | downloads the prebuilt binary for your platform (like Codex's npm package) |
| PyPI | `pypi/` | 0.0.1 prints install guidance | ships the binary in platform wheels (like ruff) |
| Homebrew | tap repo `code-with-rashid/homebrew-oxgraft` | not created | `brew install code-with-rashid/oxgraft/oxgraft` |

## Publishing 0.0.1 (maintainer, needs your own logins)

```sh
# crates.io: publish in dependency order
cargo login
cargo publish -p oxgraft-seams
cargo publish -p oxgraft-protocol
cargo publish -p oxgraft-strategies
cargo publish -p oxgraft-core
cargo publish -p oxgraft

# npm
cd packaging/npm && npm login && npm publish --access public

# PyPI
cd packaging/pypi && python -m pip install build twine && python -m build && python -m twine upload dist/*
```
