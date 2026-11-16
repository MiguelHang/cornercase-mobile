import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

const root = join(import.meta.dirname, '..')

function read(path: string) {
  return readFileSync(join(root, path), 'utf8')
}

function jsonVersion(path: string) {
  return JSON.parse(read(path)).version as string
}

function cargoVersion(text: string) {
  const section = text.split(/^\[/m).find((part) => part.startsWith('package]'))
  return section?.match(/^version\s*=\s*"([^"]+)"/m)?.[1]
}

function changelogVersion(text: string) {
  return text.match(/^## (\S+)/m)?.[1]
}

describe('cargoVersion', () => {
  it('reads the version of the package section', () => {
    expect(cargoVersion('[package]\nname = "a"\nversion = "1.2.3"\n')).toBe('1.2.3')
  })

  it('skips a version outside the package section', () => {
    const text = '[dependencies]\nversion = "9.9.9"\n\n[package]\nname = "a"\nversion = "1.2.3"\n'
    expect(cargoVersion(text)).toBe('1.2.3')
  })

  it('stops at the next section', () => {
    expect(cargoVersion('[package]\nname = "a"\n\n[dependencies]\nversion = "9.9.9"\n')).toBeUndefined()
  })
})

describe('changelogVersion', () => {
  it('reads the first version heading', () => {
    expect(changelogVersion('# Changelog\n\nIntro.\n\n## 0.2.0\n\n- b\n\n## 0.1.0\n\n- a\n')).toBe('0.2.0')
  })

  it('ignores headings of other levels', () => {
    expect(changelogVersion('# Changelog\n### 9.9.9\n## 0.1.0\n')).toBe('0.1.0')
  })

  it('finds nothing without a version heading', () => {
    expect(changelogVersion('# Changelog\n')).toBeUndefined()
  })
})

describe('the versions of the release', () => {
  const app = jsonVersion('package.json')

  it.each([
    ['src-tauri/tauri.conf.json', () => jsonVersion('src-tauri/tauri.conf.json')],
    ['src-tauri/Cargo.toml', () => cargoVersion(read('src-tauri/Cargo.toml'))],
    ['CHANGELOG.md', () => changelogVersion(read('CHANGELOG.md'))],
  ])('match package.json in %s', (_, version) => {
    expect(version()).toBe(app)
  })
})
