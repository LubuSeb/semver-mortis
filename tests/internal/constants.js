'use strict'

// Compatibility shim for unchanged upstream fixtures. This mirrors JavaScript's
// numeric boundary; all behavior under test remains in the Rust implementation.
module.exports = {
  MAX_LENGTH: 256,
  MAX_SAFE_COMPONENT_LENGTH: 16,
  MAX_SAFE_BUILD_LENGTH: 250,
  MAX_SAFE_INTEGER: Number.MAX_SAFE_INTEGER,
  RELEASE_TYPES: ['major', 'premajor', 'minor', 'preminor', 'patch', 'prepatch', 'prerelease'],
  SEMVER_SPEC_VERSION: '2.0.0',
  FLAG_INCLUDE_PRERELEASE: 0b001,
  FLAG_LOOSE: 0b010,
}
