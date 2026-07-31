'use strict'

const { spawnSync } = require('node:child_process')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const oraclePath = process.env.NODE_SEMVER_ORACLE
if (!oraclePath) {
  throw new Error('set NODE_SEMVER_ORACLE to the pinned npm/node-semver checkout')
}
const oracle = require(path.resolve(oraclePath))
const binary = process.env.SEMVER_MORTIS_BIN || path.join(
  root,
  'target',
  'debug',
  process.platform === 'win32' ? 'semver-mortis.exe' : 'semver-mortis'
)
const cases = Number(process.env.FUZZ_CASES || 2000)
let state = 0x5eedc0de
let assertions = 0
const failures = []

const random = (limit) => {
  state = (Math.imul(state, 1664525) + 1013904223) >>> 0
  return state % limit
}
const version = () => {
  let value = `${random(100)}.${random(100)}.${random(100)}`
  switch (random(5)) {
    case 0: value += '-alpha'; break
    case 1: value += `-rc.${random(20)}`; break
    case 2: value += `-${random(20)}.beta`; break
  }
  if (random(4) === 0) {
    value += `+build.${random(20)}`
  }
  return value
}
const candidate = () => {
  const valid = version()
  switch (random(7)) {
    case 0: return valid.replace(/^\d+/, value => `0${value}`)
    case 1: return valid.replace(/\.\d+\./, '..')
    case 2: return `${valid}!`
    case 3: return valid.replace(/-.*/, '-')
    default: return valid
  }
}
const range = () => {
  const major = random(8)
  const minor = random(8)
  const patch = random(8)
  switch (random(7)) {
    case 0: return `^${major}.${minor}.${patch}`
    case 1: return `~${major}.${minor}`
    case 2: return `${major}.${minor}.x`
    case 3: return `>=${major}.${minor}.${patch} <${major + 1}.0.0`
    case 4: return `${major}.${minor}.0 - ${major}.${minor + 1}.9`
    case 5: return `^${major}.${minor}.${patch}-rc.0`
    default: return `^${major}.${minor}.${patch} || ${major + 2}.x`
  }
}
const execute = (args) => spawnSync(binary, args, {
  encoding: 'utf8',
  windowsHide: true,
})
const output = (run) => (
  run.status === 0 && run.stdout.trim() !== '' ? run.stdout.trim() : null
)
const check = (operation, actual, expected, context) => {
  assertions++
  if (actual !== expected) {
    failures.push({ operation, expected, actual, ...context })
  }
}

for (let index = 0; index < cases; index++) {
  const input = candidate()
  check('valid', output(execute(['valid', input])), oracle.valid(input), { input })

  const left = version()
  const right = version()
  check(
    'compare',
    Number(output(execute(['compare', left, right]))),
    oracle.compare(left, right),
    { left, right }
  )

  const rangeText = range()
  const versionText = version()
  const includePrerelease = random(4) === 0
  const options = { includePrerelease }
  const args = includePrerelease ? ['--include-prerelease'] : []
  args.push('satisfies', versionText, rangeText)
  check(
    'satisfies',
    output(execute(args)) === 'true',
    oracle.satisfies(versionText, rangeText, options),
    { version: versionText, range: rangeText, options }
  )
}

if (failures.length) {
  console.error(JSON.stringify({ seed: '0x5eedc0de', assertions, failures }, null, 2))
  process.exitCode = 1
} else {
  console.log(`differential fuzz: ${assertions}/${assertions} generated oracle checks passed`)
}
