'use strict'

const { call } = require('./bridge')

const parseOptions = options => {
  if (!options) return {}
  if (typeof options !== 'object') return { loose: true }
  return options
}

const optionArgs = options => {
  options = parseOptions(options)
  const args = []
  if (options.loose) args.push('--loose')
  if (options.includePrerelease) args.push('--include-prerelease')
  if (options.rtl) args.push('--rtl')
  return args
}

const numeric = /^[0-9]+$/
const compareIdentifiers = (left, right) => {
  const leftNumeric = numeric.test(left)
  const rightNumeric = numeric.test(right)
  if (left === right) return 0
  if (leftNumeric && !rightNumeric) return -1
  if (rightNumeric && !leftNumeric) return 1
  if (leftNumeric && rightNumeric) {
    const a = BigInt(left)
    const b = BigInt(right)
    return a < b ? -1 : 1
  }
  return left < right ? -1 : 1
}

const rcompareIdentifiers = (left, right) => compareIdentifiers(right, left)

const decodeVersion = value => {
  const [raw, version, major, minor, patch, loose, prerelease, build] = value.split('\x1f')
  return {
    raw,
    version,
    major: Number(major),
    minor: Number(minor),
    patch: Number(patch),
    loose: loose === 'true',
    prerelease: prerelease === '' ? [] : prerelease.split(',').map(identifier => {
      const value = identifier.slice(2)
      return identifier.startsWith('n:') ? Number(value) : value
    }),
    build: build === '' ? [] : build.split(','),
  }
}

class SemVer {
  constructor (input, options) {
    options = parseOptions(options)
    if (input instanceof SemVer) {
      if (input.loose === !!options.loose && input.includePrerelease === !!options.includePrerelease) {
        return input
      }
      input = input.version
    }
    if (typeof input !== 'string') {
      throw new TypeError(`Invalid version. Must be a string. Got type "${typeof input}".`)
    }
    this.options = options
    this.includePrerelease = !!options.includePrerelease
    try {
      this._load(call([...optionArgs(options), 'inspect', input]))
    } catch (_) {
      throw new TypeError(`Invalid Version: ${input}`)
    }
  }

  _load (encoded) {
    const value = decodeVersion(encoded)
    this.raw = value.raw
    this.version = value.version
    this.major = value.major
    this.minor = value.minor
    this.patch = value.patch
    this.loose = value.loose
    this.prerelease = value.prerelease
    this.build = value.build
  }

  format () {
    this.version = `${this.major}.${this.minor}.${this.patch}`
    if (this.prerelease.length) this.version += `-${this.prerelease.join('.')}`
    return this.version
  }

  toString () { return this.version }

  compare (other) {
    other = other instanceof SemVer ? other : new SemVer(other, this.options)
    return Number(call([...optionArgs(this.options), 'compare', this.version, other.version]))
  }

  compareMain (other) {
    other = other instanceof SemVer ? other : new SemVer(other, this.options)
    return this.major !== other.major ? (this.major < other.major ? -1 : 1)
      : this.minor !== other.minor ? (this.minor < other.minor ? -1 : 1)
        : this.patch !== other.patch ? (this.patch < other.patch ? -1 : 1)
          : 0
  }

  comparePre (other) {
    other = other instanceof SemVer ? other : new SemVer(other, this.options)
    if (this.prerelease.length && !other.prerelease.length) return -1
    if (!this.prerelease.length && other.prerelease.length) return 1
    for (let index = 0; ; index++) {
      const left = this.prerelease[index]
      const right = other.prerelease[index]
      if (left === undefined && right === undefined) return 0
      if (right === undefined) return 1
      if (left === undefined) return -1
      if (left === right) continue
      return compareIdentifiers(left, right)
    }
  }

  compareBuild (other) {
    other = other instanceof SemVer ? other : new SemVer(other, this.options)
    for (let index = 0; ; index++) {
      const left = this.build[index]
      const right = other.build[index]
      if (left === undefined && right === undefined) return 0
      if (right === undefined) return 1
      if (left === undefined) return -1
      if (left === right) continue
      return compareIdentifiers(left, right)
    }
  }

  inc (release, identifier, base) {
    if (String(release).startsWith('pre')) {
      if (!identifier && base === false) {
        throw new Error('invalid increment argument: identifier is empty')
      }
      if (identifier) {
        const validIdentifier = identifier.split('.').every(part =>
          /^[0-9A-Za-z-]+$/.test(part) &&
          (this.loose || !/^0[0-9]+$/.test(part))
        )
        if (!validIdentifier) throw new Error(`invalid identifier: ${identifier}`)
      }
    }
    if (release === 'prerelease' && base === false && identifier === this.prerelease.join('.')) {
      throw new Error('invalid increment argument: identifier already exists')
    }
    const args = [...optionArgs(this.options)]
    if (typeof identifier === 'string') args.push('--identifier', identifier)
    if (base !== undefined) args.push('--identifier-base', String(base))
    args.push('inc', this.raw, release)
    const result = call(args)
    if (result === null) throw new Error(`invalid increment argument: ${release}`)
    const build = this.build.slice()
    this._load(call([...optionArgs(this.options), 'inspect', result]))
    this.build = build
    this.raw = this.version + (build.length ? `+${build.join('.')}` : '')
    return this
  }
}

const ANY = Symbol('SemVer ANY')

class Comparator {
  static get ANY () { return ANY }

  constructor (input, options) {
    options = parseOptions(options)
    if (input instanceof Comparator) {
      if (input.loose === !!options.loose) return input
      input = input.value
    }
    if (typeof input !== 'string') throw new TypeError(`Invalid comparator: ${input}`)
    input = input.trim().replace(/\s+/g, ' ')
    this.options = options
    this.loose = !!options.loose
    try {
      this.value = call([...optionArgs(options), 'comparator', input])
    } catch (_) {
      throw new TypeError(`Invalid comparator: ${input}`)
    }
    const match = this.value.match(/^(<=|>=|<|>)(.*)$/)
    this.operator = match ? match[1] : ''
    const version = match ? match[2] : this.value
    this.semver = version ? new SemVer(version, options) : ANY
  }

  toString () { return this.value }

  test (version) {
    if (this.semver === ANY || version === ANY) return true
    try {
      version = version instanceof SemVer ? version : new SemVer(version, this.options)
    } catch (_) {
      return false
    }
    return cmp(version, this.operator, this.semver, this.options)
  }

  intersects (other, options) {
    if (!(other instanceof Comparator)) throw new TypeError('a Comparator is required')
    return call([...optionArgs(options), 'intersects', this.value, other.value]) === 'true'
  }
}

const rangeCache = new Map()

class Range {
  constructor (input, options) {
    options = parseOptions(options)
    if (input instanceof Range) {
      if (input.loose === !!options.loose && input.includePrerelease === !!options.includePrerelease) return input
      input = input.raw
    }
    if (input instanceof Comparator) {
      this.raw = input.value
      this.set = [[input]]
      this.options = options
      this.loose = !!options.loose
      this.includePrerelease = !!options.includePrerelease
      this.formatted = undefined
      return
    }
    if (typeof input !== 'string') throw new TypeError(`Invalid SemVer Range: ${input}`)
    this.options = options
    this.loose = !!options.loose
    this.includePrerelease = !!options.includePrerelease
    this.raw = input.trim().replace(/\s+/g, ' ')
    const cacheKey = `${this.loose ? 1 : 0}:${this.includePrerelease ? 1 : 0}:${this.raw}`
    const cached = rangeCache.get(cacheKey)
    if (cached) {
      this.set = cached
    } else {
      let canonical
      try {
        canonical = call([...optionArgs(options), 'range', this.raw])
      } catch (_) {
        throw new TypeError(`Invalid SemVer Range: ${this.raw}`)
      }
      this.set = canonical === ''
        ? [[new Comparator('', options)]]
        : canonical.split('||').map(group => group.split(' ').map(value => new Comparator(value, options)))
      rangeCache.set(cacheKey, this.set)
    }
    this.formatted = undefined
  }

  get range () {
    if (this.formatted === undefined) {
      this.formatted = this.set.map(group => group.map(String).join(' ')).join('||')
    }
    return this.formatted
  }

  format () { return this.range }
  toString () { return this.range }

  test (version) {
    if (!version) return false
    const value = version instanceof SemVer ? version.version : version
    return call([...optionArgs(this.options), 'satisfies', value, this.raw]) === 'true'
  }

  intersects (other, options) {
    if (!(other instanceof Range)) throw new TypeError('a Range is required')
    return call([...optionArgs(options), 'intersects', this.raw, other.raw]) === 'true'
  }
}

const parse = (input, options, throwErrors = false) => {
  if (input instanceof SemVer) return input
  try { return new SemVer(input, options) } catch (error) {
    if (throwErrors) throw error
    return null
  }
}
const valid = (input, options) => parse(input, options)?.version || null
const clean = (input, options) => {
  if (typeof input !== 'string') return null
  try { return call([...optionArgs(options), 'clean', input]) } catch (_) { return null }
}
const compare = (left, right, options) => new SemVer(left, options).compare(new SemVer(right, options))
const rcompare = (left, right, options) => compare(right, left, options)
const compareLoose = (left, right) => compare(left, right, true)
const compareBuild = (left, right, options) => {
  const a = new SemVer(left, options)
  const b = new SemVer(right, options)
  return a.compare(b) || a.compareBuild(b)
}
const eq = (left, right, options) => compare(left, right, options) === 0
const neq = (left, right, options) => compare(left, right, options) !== 0
const gt = (left, right, options) => compare(left, right, options) > 0
const gte = (left, right, options) => compare(left, right, options) >= 0
const lt = (left, right, options) => compare(left, right, options) < 0
const lte = (left, right, options) => compare(left, right, options) <= 0
const cmp = (left, operator, right, options) => {
  switch (operator) {
    case '===':
      if (typeof left === 'object') left = left.version
      if (typeof right === 'object') right = right.version
      return left === right
    case '!==':
      if (typeof left === 'object') left = left.version
      if (typeof right === 'object') right = right.version
      return left !== right
    case '': case '=': case '==': return eq(left, right, options)
    case '!=': return neq(left, right, options)
    case '>': return gt(left, right, options)
    case '>=': return gte(left, right, options)
    case '<': return lt(left, right, options)
    case '<=': return lte(left, right, options)
    default: throw new TypeError(`Invalid operator: ${operator}`)
  }
}
const inc = (input, release, options, identifier, base) => {
  try {
    const version = new SemVer(input, options)
    return new SemVer(version.version, options).inc(release, identifier, base).version
  } catch (_) { return null }
}
const diff = (left, right) => {
  const a = new SemVer(left)
  const b = new SemVer(right)
  return call(['diff', a.version, b.version])
}
const major = (input, options) => new SemVer(input, options).major
const minor = (input, options) => new SemVer(input, options).minor
const patch = (input, options) => new SemVer(input, options).patch
const prerelease = (input, options) => {
  const value = parse(input, options)
  return value && value.prerelease.length ? value.prerelease : null
}
const sort = (values, options) => values.sort((left, right) => compareBuild(left, right, options))
const rsort = (values, options) => values.sort((left, right) => compareBuild(right, left, options))
const truncate = (input, release, options) => {
  try { return call([...optionArgs(options), 'truncate', String(input), release]) } catch (_) { return null }
}
const coerce = (input, options) => {
  if (input instanceof SemVer) return input
  if (typeof input !== 'string' && typeof input !== 'number') return null
  try {
    const value = call([...optionArgs(options), 'coerce-full', String(input)])
    return value === null ? null : new SemVer(value)
  } catch (_) { return null }
}
const satisfies = (version, range, options) => {
  try { return new Range(range, options).test(version) } catch (_) { return false }
}
const validRange = (input, options) => {
  try { return new Range(input, options).range || '*' } catch (_) { return null }
}
const minVersion = (input, options) => {
  const range = new Range(input, options)
  const value = call([...optionArgs(options), 'min-version', range.raw])
  return value === null ? null : new SemVer(value)
}
const extreme = (command, versions, input, options) => {
  const range = new Range(input, options)
  return call([...optionArgs(options), command, range.raw, ...versions.map(String)])
}
const maxSatisfying = (versions, range, options) => {
  try { return extreme('max-satisfying', versions, range, options) } catch (_) { return null }
}
const minSatisfying = (versions, range, options) => {
  try { return extreme('min-satisfying', versions, range, options) } catch (_) { return null }
}
const outside = (version, range, hilo, options) => {
  if (hilo !== '>' && hilo !== '<') throw new TypeError('Must provide a hilo val of "<" or ">"')
  return call([...optionArgs(options), hilo === '>' ? 'gtr' : 'ltr', String(version), String(range)]) === 'true'
}
const gtr = (version, range, options) => outside(version, range, '>', options)
const ltr = (version, range, options) => outside(version, range, '<', options)
const intersects = (left, right, options) => call([
  ...optionArgs(options), 'intersects', String(left), String(right),
]) === 'true'
const subset = (sub, domain, options) => call([
  ...optionArgs(options), 'subset', String(sub), String(domain),
]) === 'true'
const toComparators = (input, options) => new Range(input, options).set.map(set => set.map(String))
const simplify = (versions, input, options) => {
  const ordered = versions.slice().sort((left, right) => compare(left, right, options))
  const groups = []
  let first = null
  let previous = null
  for (const version of ordered) {
    if (satisfies(version, input, options)) {
      previous = version
      first ||= version
    } else if (previous) {
      groups.push([first, previous])
      first = previous = null
    }
  }
  if (first) groups.push([first, null])
  const simplified = groups.map(([min, max]) => min === max ? min
    : !max && min === ordered[0] ? '*'
      : !max ? `>=${min}`
        : min === ordered[0] ? `<=${max}`
          : `${min} - ${max}`).join(' || ')
  const raw = input instanceof Range ? input.raw : String(input)
  return simplified.length < raw.length ? simplified : input
}

module.exports = {
  SemVer, Comparator, Range, parseOptions, compareIdentifiers, rcompareIdentifiers,
  parse, valid, clean, compare, rcompare, compareLoose, compareBuild, eq, neq, gt, gte, lt, lte,
  cmp, inc, diff, major, minor, patch, prerelease, sort, rsort, truncate, coerce, satisfies,
  validRange, minVersion, maxSatisfying, minSatisfying, outside, gtr, ltr, intersects, subset,
  toComparators, simplify,
}
