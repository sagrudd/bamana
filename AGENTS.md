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
