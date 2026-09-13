# oxr

`oxr` is a semantic-version bump and git-tag orchestrator for repos that have
no package manifest and no single programming language — GitHub Actions
composite actions/reusable workflows, and manifest-bearing repos (like a
Claude Code plugin) that still want their version sourced from git tags.

Git tags are the **only** source of truth for the current version — there is
no manifest field oxr reads. Version comparisons use real semver precedence
(via the `semver` crate), not lexicographic sorting and not git's native tag
sort.

## Install

### As a GitHub Action

```yaml
permissions:
  contents: write # tag/push operations 403 without this

steps:
  - uses: actions/checkout@v4
    with:
      fetch-depth: 0 # required: the default fetch-depth: 1 doesn't fetch tags

  - uses: oxhive/oxr@v1

  - run: oxr release patch --execute
```

`oxr` refuses to run against a shallow checkout with a clear error rather
than silently miscalculating the version, so a missing `fetch-depth: 0` is
caught immediately.

Prebuilt binaries are committed directly into this repo under `dist/`
(see `dist/README.md`) rather than published as GitHub Release assets, so
pinning the action to a floating tag (`@v1`) always resolves to the matching
binary with zero extra fetch step. Supported platforms: `Linux-X64`,
`Linux-ARM64`, `macOS-ARM64`.

### Locally

```sh
cargo install --path .
```

## CLI

```
oxr init [--force]
oxr release <level> [--for <major|minor>] [--execute]
oxr float --tag <tag> [--execute]
oxr current [--json]
```

Both `release` and `float` are dry-run by default — they print their plan
and make no changes until `--execute` is passed.

### `oxr init`

Writes a `release.toml` scaffold to the repo root, with every setting
commented out and set to its default value. A repo running the scaffold
as-is behaves identically to having no config file at all — `init` never
silently changes release behavior (for example, activating a
`pre-release-replacements` entry against a file that doesn't exist yet
would break the next release). Uncomment and edit only what you need to
change.

Refuses to overwrite an existing `release.toml` unless `--force` is
passed. Unlike the other subcommands, `init` works fine against a shallow
checkout, since it doesn't need tag history.

### `oxr current`

Read-only. Prints the two distinct resolution queries oxr makes over tag
state:

- `latest_stable` — the highest semver tag with no pre-release component.
- `active_train` — the latest pre-release tag overall, if one exists.

Run this before `release`/`execute` when it isn't obvious what either
command will actually do.

### `oxr release <level>`

| Level | Behavior |
|---|---|
| `patch` / `minor` / `major` | Bump the given component from `latest_stable`. |
| `stable` | Finalize the active pre-release train (`1.5.0-rc.3` → `1.5.0`). Errors if no train is active. |
| `alpha` / `beta` / `rc` | Start or advance a pre-release train (see below). |

**Pre-release trains.** A train is active whenever the highest-precedence
tag overall carries a pre-release component — this is derived purely from
tag state, nothing is stored. Starting a fresh train defaults to bumping
`patch` (override with `--for major` or `--for minor`); on the *same*
version target, re-running the same stage increments its counter
(`rc.2` → `rc.3`), while moving to a more mature stage resets the counter
(`alpha.3` → `beta.1`). Maturity only moves forward: releasing `alpha` on
top of an existing `beta` or `rc` (or `beta` on top of `rc`) is an error.
`--for` on an already-active train must match the train's existing target
or oxr errors rather than silently ignoring it.

**Bootstrap.** With zero matching tags, the implicit baseline is `0.0.0`
and oxr always targets a minor bump regardless of the requested level —
the first release is `0.1.0`, not `0.0.1` or `1.0.0` — unless `--for`
explicitly overrides it. To start at `1.0.0` directly with no override,
tag it manually first: `git tag v1.0.0`.

**Known edge case.** Because semver precedence compares
`major.minor.patch` before pre-release status, a shelved pre-release tag
from an old planning cycle (e.g. `v1.5.0-rc.1` cut for a minor release that
never shipped, with current stable still at `v1.4.2`) will still resolve as
an active train. There's no override flag for this by design — delete the
orphaned tag, the same way you'd delete a stale branch.

### `oxr float --tag <tag>`

Moves the configured floating major/minor tag(s) (`v1`, `v1.2`, ...) to
point at `tag`. Hard-refuses if `tag` carries a pre-release identifier —
floating tags exist so consumers pinning to `@v1` get trusted, stable code,
and this check lives in the binary itself, not just in CI trigger wiring.

A major bump only ever creates the new major's floating tag; the previous
major's floating tag is never touched.

Recommended: run `float` from its own CI job, gated on the release tag's
test/build suite passing, with its own scoped credentials — never from
developer machines:

```yaml
on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+' # excludes v*-rc.*, v*-alpha.*, etc.

jobs:
  float:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: oxhive/oxr@v1
      # ...run the repo's test/build suite here; only float on success...
      - run: oxr float --tag ${{ github.ref_name }} --execute
```

## Configuration

Read from `release.toml` at the repo root by default, falling back to
`oxr.toml`. Neither file existing is fine — oxr runs on defaults.

```toml
sign-commit = false
sign-tag = false
push = true
tag = true
tag-name = "v{{version}}"
tag-pattern = "^v\\d+\\.\\d+\\.\\d+"
pre-release-commit-message = "chore: release v{{version}}"

[float-tags]
major = true
minor = false
major-tag-name = "v{{major}}"
minor-tag-name = "v{{major}}.{{minor}}"

[[pre-release-replacements]]
file = ".claude-plugin/plugin.json"
search = "\"version\": \"[^\"]+\""
replace = "\"version\": \"{{version}}\""
exactly = 1
```

- `tag-pattern` filters which existing tags oxr considers during version
  resolution — any tag not matching it is ignored entirely. It's decoupled
  from `tag-name`, which governs the format of *new* tags.
- `pre-release-replacements` entries fail loudly if `search` doesn't match
  the file exactly `exactly` times, so a drifted regex can't silently
  no-op. `replace` is rendered for `{{version}}`/`{{major}}`/`{{minor}}`/
  `{{patch}}` first, then applied as a regex replacement — so it also
  supports backreferences (`$1`, `$2`, ...) to preserve parts of the
  original match.
- Template variables: `{{version}}` (includes any pre-release suffix, e.g.
  `1.5.0-rc.1`), `{{major}}`, `{{minor}}`, `{{patch}}`.

## Out of scope

See the handover spec for the full reasoning. In short, oxr deliberately
does not have: a `publish` command, a `tag-prefix` field (the `v` lives in
`tag-name`), a post-release "dev version" bump, self-reference version
bumping (GitHub's `$/` syntax already solves that), workspace/multi-crate
release ordering, or a floating "staging" pointer like `@next`.
