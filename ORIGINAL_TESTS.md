# Unchanged upstream test proof

## Result

```text
verified 66 byte-identical upstream files
49/49 unchanged non-CLI suites passed
8,787/8,787 assertions passed
```

The source is `npm/node-semver` 7.8.5 at commit
`6e05b7637396ac66522cff8731f07cfe0ef49a29`. All 66 copied JavaScript files
remain under `tests/original/`; `npm run test:original` verifies every SHA-256
digest before executing a suite.

## Included unchanged suites

| Area | Suites | What they cover |
| --- | ---: | --- |
| Version and comparator classes | 4 | construction, normalization, precedence, mutation, caching, intersections |
| Functional API | 24 | parse/clean/compare/inc/coerce/diff/sort/truncate and helpers |
| Range API | 12 | grammar, satisfy, extrema, outside, intersect, simplify, subset |
| Internal contracts | 6 | constants, identifiers, options, LRU, debug, reflection types |
| Entry points | 2 | main and preload export contracts |
| Long-input integration | 1 | 500,000-character whitespace and zero-input regressions |
| **Total** | **49** | **8,787 assertions** |

The unchanged suites resolve their original CommonJS paths to adapter modules
outside `tests/original/`. Those modules forward semantic operations to the
compiled Rust binary. Original fixture imports remain original fixture imports.

## Excluded upstream suites

There are 51 non-fixture JavaScript test files. Two are deliberately not
counted:

- `bin/semver.js` snapshot-tests the original Node CLI and its exact help text.
  The port ships a native command-oriented Rust CLI, covered directly by four
  integration tests in `tests/cli.rs`.
- `map.js` checks one-to-one JavaScript source/test filename topology and npm
  package metadata. It describes the old repository layout, not SemVer
  behavior.

No failure, skip, todo, or edited original test is included in the 49/49 claim.

## Reproduce

```sh
npm ci
npm run test:original
```

The pinned `tap@16.3.10` is the exact version installed in the upstream oracle
checkout. It is development-only and never enters the Rust artifact.
