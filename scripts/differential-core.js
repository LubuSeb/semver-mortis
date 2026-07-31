'use strict'

const { spawnSync } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const binary = process.env.SEMVER_MORTIS_BIN || path.join(
  root,
  'target',
  'debug',
  process.platform === 'win32' ? 'semver-mortis.exe' : 'semver-mortis'
)
const fixtures = path.join(root, 'tests', 'original', 'fixtures')
const failures = []
let assertions = 0

const optionsArgs = (options) => (
  options === true || options?.loose ? ['--loose'] : []
)
const execute = (args) => spawnSync(binary, args, {
  encoding: 'utf8',
  windowsHide: true,
})
const output = (run) => (
  run.status === 0 && run.stdout.trim() !== '' ? run.stdout.trim() : null
)
const check = (suite, actual, expected, context) => {
  assertions++
  if (actual !== expected) {
    failures.push({ suite, ...context, expected, actual })
  }
}

for (const [left, right, options] of require(path.join(fixtures, 'comparisons.js'))) {
  check(
    'comparisons',
    output(execute([...optionsArgs(options), 'compare', left, right])),
    '1',
    { left, right, options }
  )
}

for (const [left, right, options] of require(path.join(fixtures, 'equality.js'))) {
  check(
    'equality',
    output(execute([...optionsArgs(options), 'compare', left, right])),
    '0',
    { left, right, options }
  )
}

for (const [version, major, minor, patch, prerelease] of require(path.join(fixtures, 'valid-versions.js'))) {
  const expected = `${major}.${minor}.${patch}${prerelease.length ? `-${prerelease.join('.')}` : ''}`
  check('valid-versions', output(execute(['valid', version])), expected, { version })
}

for (const [value, reason, options] of require(path.join(fixtures, 'invalid-versions.js'))) {
  if (typeof value === 'string') {
    check(
      'invalid-version-strings',
      output(execute([...optionsArgs(options), 'valid', value])),
      null,
      { value, reason, options }
    )
  }
}

for (const [version, release, expected, options, identifier, base] of require(path.join(fixtures, 'increments.js'))) {
  const args = [...optionsArgs(options)]
  if (typeof identifier === 'string') {
    args.push('--identifier', identifier)
  }
  if (base !== undefined) {
    args.push('--identifier-base', String(base))
  }
  args.push('inc', version, release)
  check('increments', output(execute(args)), expected, {
    version, release, options, identifier, base,
  })
}

for (const [version, release, expected] of require(path.join(fixtures, 'truncations.js'))) {
  check(
    'truncations',
    output(execute(['truncate', version, release])),
    expected,
    { version, release }
  )
}

const coerceSource = fs.readFileSync(
  path.join(root, 'tests', 'original', 'functions', 'coerce.js'),
  'utf8'
)
const extractCoerceCases = (name) => {
  const expression = new RegExp(
    `const ${name} = (\\[[\\s\\S]*?\\r?\\n  \\])\\r?\\n  ${name}\\.forEach`
  )
  const match = coerceSource.match(expression)
  if (!match) {
    throw new Error(`could not locate unchanged ${name} cases`)
  }
  return vm.runInNewContext(`(${match[1]})`, { parse: value => value })
}
const coerceOutput = (input, options = {}) => {
  const args = []
  if (options?.rtl) {
    args.push('--rtl')
  }
  if (options?.includePrerelease) {
    args.push('--include-prerelease')
  }
  args.push('coerce-full', String(input))
  return output(execute(args))
}

for (const input of extractCoerceCases('coerceToNull')) {
  if (typeof input === 'string' || typeof input === 'number') {
    check('coerce-invalid', coerceOutput(input), null, { input })
  }
}
for (const [input, expected, options] of extractCoerceCases('coerceToValid')) {
  if (typeof input === 'string' || typeof input === 'number') {
    check('coerce-valid', coerceOutput(input, options), expected, { input, options })
  }
}

if (failures.length) {
  console.error(JSON.stringify({ assertions, failures }, null, 2))
  process.exitCode = 1
} else {
  console.log(`core differential: ${assertions}/${assertions} upstream fixtures passed`)
}
