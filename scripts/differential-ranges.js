'use strict'

const { spawnSync } = require('node:child_process')
const path = require('node:path')

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

const optionsArgs = (options) => {
  const args = []
  if (options === true || options?.loose) {
    args.push('--loose')
  }
  if (options?.includePrerelease) {
    args.push('--include-prerelease')
  }
  return args
}

const execute = (args) => spawnSync(binary, args, {
  encoding: 'utf8',
  windowsHide: true,
})

for (const [input, expected, options = {}] of require(path.join(fixtures, 'range-parse.js'))) {
  const run = execute([...optionsArgs(options), 'range', input])
  let actual = run.status === 0 ? run.stdout.replace(/\r?\n$/, '') : null
  actual = actual === '' ? '*' : actual
  assertions++
  if (actual !== expected) {
    failures.push({ suite: 'range-parse', input, expected, actual, options })
  }
}

for (const [file, expected] of [
  ['range-include.js', true],
  ['range-exclude.js', false],
]) {
  for (const [range, version, options = {}] of require(path.join(fixtures, file))) {
    const run = execute([...optionsArgs(options), 'satisfies', version, range])
    const actual = run.status === 0 && run.stdout.trim() === 'true'
    assertions++
    if (actual !== expected) {
      failures.push({ suite: file, range, version, expected, actual, options })
    }
  }
}

if (failures.length) {
  console.error(JSON.stringify({ assertions, failures }, null, 2))
  process.exitCode = 1
} else {
  console.log(`range differential: ${assertions}/${assertions} upstream fixtures passed`)
}
