'use strict'

module.exports = class LRUCache {
  constructor () {
    this.max = 1000
    this.map = new Map()
  }
  get (key) {
    const value = this.map.get(key)
    if (value === undefined) return undefined
    this.map.delete(key)
    this.map.set(key, value)
    return value
  }
  delete (key) { return this.map.delete(key) }
  set (key, value) {
    const deleted = this.delete(key)
    if (!deleted && value !== undefined) {
      if (this.map.size >= this.max) this.delete(this.map.keys().next().value)
      this.map.set(key, value)
    }
    return this
  }
}
