'use strict'

// Compatibility shim for unchanged upstream fixtures. This mirrors JavaScript's
// numeric boundary; all behavior under test remains in the Rust implementation.
module.exports = {
  MAX_LENGTH: 256,
  MAX_SAFE_INTEGER: Number.MAX_SAFE_INTEGER,
}
