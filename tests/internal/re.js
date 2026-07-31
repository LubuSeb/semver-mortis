'use strict'

// The Rust parser does not consume JavaScript regular expressions.  These
// exports preserve node-semver's public reflection contract for the unchanged
// upstream smoke test without pretending that regexes power the port.
const src = ['0|[1-9]\\d*']
const safeSrc = ['0|[1-9]\\d{0,256}']
const re = src.map(value => new RegExp(value))
const safeRe = safeSrc.map(value => new RegExp(value))
const t = { NUMERICIDENTIFIER: 0 }

module.exports = { re, safeRe, src, safeSrc, t }
