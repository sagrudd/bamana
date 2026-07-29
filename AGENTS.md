# Repository agent instructions

## Kanon identity contract

- This repository's Mnemosyne product or component identity must be registered
  in the authoritative Kanon registry at
  `https://github.com/sagrudd/kanon`.
- Changes to the stable identifier, display name, repository location, crate,
  package, container, binary, product-manifest or schema coordinates, supported
  host modes, dependencies, compatibility, lifecycle, aliases, deprecation, or
  replacement must include the corresponding Kanon change in the same delivery
  transaction or an explicitly linked Kanon pull request.
- Before a release, verify that this repository's Kanon identity and dependency
  declarations match the release artefacts. Once Kanon channels and locksets
  are operational, releases and maintained product branches must use the
  applicable supported channel and pin the resolved lockset identifier and
  digest.
- Do not invent, rename, or reuse Mnemosyne product identifiers locally. Do not
  treat registration in Kanon as proof that a component is installed,
  entitled, healthy, or supported by every host profile.
- If live Kanon services are unavailable, use a verified pinned Kanon snapshot
  or lockset. Do not bypass identity or compatibility validation to make a
  release proceed.

## Shared release and change discipline

- Follow Semantic Versioning for every release-facing crate, package, binary,
  container, schema, manifest, API, and other independently versioned
  compatibility contract. Use `PATCH` for compatible fixes, `MINOR` for
  backwards-compatible capability, and `MAJOR` for incompatible change.
- A major version bump must not be implemented, tagged, or published without
  explicit approval from a human reviewer or the project owner.
- Every completed code change must be committed and pushed to the configured
  remote before the task is considered complete. Material documentation,
  schema, configuration, test, packaging, and operational-policy changes must
  be committed and pushed as well.
- Before committing, inspect status and diffs, stage only the intended paths,
  and preserve unrelated user or agent work. Never reset, overwrite, stash, or
  reformat unrelated changes merely to obtain a clean tree.
- Run the narrowest relevant formatting, lint, test, build, documentation, and
  compatibility checks before pushing. Do not knowingly push a failing change
  unless a human reviewer explicitly accepts the exception; report any check
  that could not be run.
- Update public documentation, contracts, compatibility or migration guidance,
  changelog/release notes, and version metadata together with the behavior they
  describe.
- Never commit secrets, credentials, private keys, production personal data,
  generated caches, or unreviewed large/binary artefacts. Preserve lockfiles,
  checksums, provenance, and dependency evidence required for reproducible
  releases.
- If commit, push, or required verification is blocked, report the precise
  blocker and any files or commits left outstanding rather than claiming the
  task is complete.
