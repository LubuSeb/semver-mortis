# Upstream test provenance

- Upstream repository: `https://github.com/npm/node-semver`
- Upstream commit: `6e05b7637396ac66522cff8731f07cfe0ef49a29`
- Snapshot time: 2026-07-31 after the 18:00 UTC kickoff
- Original files: 66 under `tests/original/`
- Copy verification: 66/66 canonical repository files byte-identical to the pinned upstream commit
- Canonical `SHA256SUMS` digest: `de26d42f3023955faed954be16ee80e926c07423f213b995ca5159b342a42ca5`

The manifest is defined over the upstream repository's LF form. The verifier
normalizes a Windows CRLF checkout before hashing, and `.gitattributes` pins
future checkouts of the original corpus to LF.

The original test files must not be edited. Adapter code lives outside `tests/original/`.
