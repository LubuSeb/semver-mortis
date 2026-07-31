'use strict'

const { createHash } = require('node:crypto')
const { readdirSync, readFileSync } = require('node:fs')
const { join, resolve } = require('node:path')
const { spawnSync } = require('node:child_process')

const root = resolve(__dirname, '..')
const original = join(root, 'tests', 'original')

const manifest = readFileSync(join(root, 'tests', 'SHA256SUMS'), 'utf8')
  .trim()
  .split(/\r?\n/)

for (const line of manifest) {
  const [, expected, relative] = line.match(/^([0-9a-f]{64})  (.+)$/) || []
  if (!expected) throw new Error(`invalid SHA256SUMS line: ${line}`)
  const source = readFileSync(join(original, relative), 'utf8').replace(/\r\n/g, '\n')
  const actual = createHash('sha256').update(source).digest('hex')
  if (actual !== expected) throw new Error(`upstream test changed: ${relative}`)
}

const files = [
  join(original, 'index.js'),
  join(original, 'preload.js'),
  ...['classes', 'functions', 'internal', 'ranges'].flatMap(directory =>
    readdirSync(join(original, directory))
      .filter(file => file.endsWith('.js'))
      .sort()
      .map(file => join(original, directory, file))
  ),
  join(original, 'integration', 'whitespace.js'),
]

console.log(`verified ${manifest.length} byte-identical upstream files`)
console.log(`running ${files.length} unchanged non-CLI suites through the Rust adapter`)

const tap = require.resolve('tap/bin/run.js')
const result = spawnSync(process.execPath, [tap, '--no-coverage', '--reporter=terse', ...files], {
  cwd: root,
  stdio: 'inherit',
})

if (result.error) throw result.error
process.exitCode = result.status === null ? 1 : result.status
