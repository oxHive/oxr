Prebuilt `oxr` binaries, one per `${{ runner.os }}-${{ runner.arch }}` pair,
committed directly into this repo and picked up by `action.yml`. See
`.github/workflows/build-binaries.yml`, which rebuilds and commits these on
every push to `main` that touches `src/`, `Cargo.toml`, or `Cargo.lock`.

Supported platforms: `Linux-X64`, `Linux-ARM64`, `macOS-ARM64`.

`Linux-ARM64/` and `macOS-ARM64/` are populated by the first CI run on
GitHub-hosted `ubuntu-24.04-arm` and `macos-14` runners rather than committed
from this environment (no cross-toolchain for either target was available
here) — `Linux-X64/oxr` was built and committed directly.
