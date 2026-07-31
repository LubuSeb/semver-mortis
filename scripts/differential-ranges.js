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

for (const [file, command, expected] of [
  ['version-gt-range.js', 'gtr', true],
  ['version-not-gt-range.js', 'gtr', false],
  ['version-lt-range.js', 'ltr', true],
  ['version-not-lt-range.js', 'ltr', false],
]) {
  for (const [range, version, options = {}] of require(path.join(fixtures, file))) {
    const run = execute([...optionsArgs(options), command, version, range])
    const actual = run.status === 0 && run.stdout.trim() === 'true'
    assertions++
    if (actual !== expected) {
      failures.push({ suite: file, range, version, expected, actual, options })
    }
  }
}

for (const [left, right, expected] of require(path.join(fixtures, 'range-intersection.js'))) {
  const run = execute(['intersects', left, right])
  const actual = run.status === 0 && run.stdout.trim() === 'true'
  assertions++
  if (actual !== expected) {
    failures.push({ suite: 'range-intersection', left, right, expected, actual })
  }
}

const subsetSource = fs.readFileSync(
  path.join(root, 'tests', 'original', 'ranges', 'subset.js'),
  'utf8'
)
const subsetMatch = subsetSource.match(
  /const cases = (\[[\s\S]*?\r?\n\])\r?\n\r?\nt\.plan/
)
if (!subsetMatch) {
  throw new Error('could not locate unchanged subset cases')
}
const subsetCases = vm.runInNewContext(`(${subsetMatch[1]})`)
for (const [sub, domain, expected, options = {}] of subsetCases) {
  const run = execute([...optionsArgs(options), 'subset', sub, domain])
  const actual = run.status === 0 && run.stdout.trim() === 'true'
  assertions++
  if (actual !== expected) {
    failures.push({ suite: 'subset', sub, domain, expected, actual, options })
  }
}

if (failures.length) {
  console.error(JSON.stringify({ assertions, failures }, null, 2))
  process.exitCode = 1
} else {
  console.log(`range differential: ${assertions}/${assertions} upstream fixtures passed`)
}
