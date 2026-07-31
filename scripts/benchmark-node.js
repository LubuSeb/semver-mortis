'use strict'

const path = require('node:path')

const oraclePath = process.env.NODE_SEMVER_ORACLE
if (!oraclePath) {
  throw new Error('set NODE_SEMVER_ORACLE to a checkout of npm/node-semver')
}
const semver = require(path.resolve(oraclePath))
const iterations = 1_000_000
const samples = 5
let sink

const benchmark = (name, operation) => {
  for (let index = 0; index < 10_000; index++) {
    sink = operation()
  }
  const timings = []
  for (let sample = 0; sample < samples; sample++) {
    const start = process.hrtime.bigint()
    for (let index = 0; index < iterations; index++) {
      sink = operation()
    }
    timings.push(Number(process.hrtime.bigint() - start) / iterations)
  }
  timings.sort((left, right) => left - right)
  const nanoseconds = timings[Math.floor(samples / 2)]
  const operationsPerSecond = 1_000_000_000 / nanoseconds
  console.log(
    `${name.padEnd(12)} ${nanoseconds.toFixed(1).padStart(10)} ns/op  ` +
    `${operationsPerSecond.toFixed(0).padStart(12)} ops/s  (median of ${samples})`
  )
}

benchmark('parse strict', () => semver.parse('2.17.4-rc.12+build.99'))

const left = new semver.SemVer('2.17.4-rc.12+build.99')
const right = new semver.SemVer('2.17.4')
benchmark('compare', () => left.compare(right))

const range = new semver.Range('^2.4.0 || >=3.1.0 <4.0.0')
benchmark('range test', () => range.test('3.5.7-rc.1'))
