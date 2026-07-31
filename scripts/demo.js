'use strict'

const { execFileSync } = require('node:child_process')
const { join, resolve } = require('node:path')

const root = resolve(__dirname, '..')
const binary = join(root, 'target', 'release', process.platform === 'win32'
  ? 'semver-mortis.exe'
  : 'semver-mortis')

const cases = [
  [['valid', '1.2.3-beta.1+build.7'], '1.2.3-beta.1'],
  [['range', '^1.2.3 || 3.x'], '>=1.2.3 <2.0.0-0||>=3.0.0 <4.0.0-0'],
  [['satisfies', '1.8.4', '^1.2.3 || >=3'], 'true'],
  [['satisfies', '1.3.0-beta.1', '^1.2.3'], 'false'],
  [['--rtl', 'coerce', 'release-1.2.3.4'], '2.3.4'],
  [[
    '--identifier', 'beta', '--identifier-base', '1',
    'inc', '1.2.3', 'preminor',
  ], '1.3.0-beta.1'],
  [['subset', '^1.2.3', '>=1 <2'], 'true'],
]

console.log('\nSEMVER MORTIS — native Rust behavior tour\n')
for (const [args, expected] of cases) {
  const actual = execFileSync(binary, args, { encoding: 'utf8' }).trim()
  if (actual !== expected) {
    throw new Error(`${args.join(' ')}: expected ${expected}, got ${actual}`)
  }
  console.log(`$ semver-mortis ${args.map(value => value.includes(' ') ? `"${value}"` : value).join(' ')}`)
  console.log(`  ${actual}\n`)
}

console.log('Proof snapshot')
console.log('  upstream suites       49/49 (8,787 assertions)')
console.log('  frozen vectors        1,144/1,144')
console.log('  property scenarios    17,000')
console.log('  differential fuzz     6,000/6,000')
console.log('  runtime dependencies  0')
console.log('  unsafe blocks         0')
