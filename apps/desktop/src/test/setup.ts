import '@testing-library/jest-dom'

const hasCompleteStorage =
  typeof globalThis.localStorage?.getItem === 'function' &&
  typeof globalThis.localStorage?.setItem === 'function' &&
  typeof globalThis.localStorage?.removeItem === 'function' &&
  typeof globalThis.localStorage?.clear === 'function'

if (!hasCompleteStorage) {
  const values = new Map<string, string>()

  const storage: Storage = {
    get length() {
      return values.size
    },
    clear: () => {
      values.clear()
    },
    getItem: (key: string) => values.get(key) ?? null,
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    removeItem: (key: string) => {
      values.delete(key)
    },
    setItem: (key: string, value: string) => {
      values.set(key, value)
    },
  }

  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: storage,
  })

  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: storage,
  })
}
