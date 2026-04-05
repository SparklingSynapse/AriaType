import { cpSync, mkdirSync, existsSync, readdirSync, writeFileSync, readFileSync, rmSync } from 'fs'
import { join, extname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const targetBase = join(__dirname, '../apps/desktop/src-tauri/target')
const destDir = join(__dirname, '../packages/website/public/release')

mkdirSync(destDir, { recursive: true })

const rootCandidates = readdirSync(targetBase, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => join(targetBase, entry.name, 'release/bundle'))
const bundleCandidates = [join(targetBase, 'release/bundle'), ...rootCandidates]
const bundleDirs = Array.from(new Set(bundleCandidates)).filter((dir) => existsSync(dir))

if (bundleDirs.length === 0) {
  console.log('No bundle directory found, skipping installer copy.')
  process.exit(0)
}

const installerExts = new Set(['.dmg', '.exe', '.msi'])
const installerDirs = ['dmg', 'nsis', 'msi']
const copiedFiles = new Set()
const artifacts = []

function detectMacArch(file) {
  const name = file.toLowerCase()
  if (name.includes('aarch64') || name.includes('arm64')) return 'mac-aarch'
  if (name.includes('x86_64') || name.includes('x64') || name.includes('intel')) return 'mac-intel'
  if (name.includes('universal')) return 'mac-universal'
  return 'mac'
}

// Map to track the latest installer per channel
const latestByChannel = new Map()

for (const bundleDir of bundleDirs) {
  for (const dir of installerDirs) {
    const srcDir = join(bundleDir, dir)
    if (!existsSync(srcDir)) continue

    for (const file of readdirSync(srcDir)) {
      if (!installerExts.has(extname(file))) continue
      
      const lower = file.toLowerCase()
      let channel = 'unknown'
      if (lower.endsWith('.dmg')) channel = detectMacArch(file)
      if (lower.endsWith('.exe')) channel = 'win-exe'
      if (lower.endsWith('.msi')) channel = 'win-msi'

      // Clean up previous files for this channel in destDir
      const existingFiles = readdirSync(destDir)
      for (const existing of existingFiles) {
        if (!installerExts.has(extname(existing))) continue
        
        let existingChannel = 'unknown'
        const existingLower = existing.toLowerCase()
        if (existingLower.endsWith('.dmg')) existingChannel = detectMacArch(existing)
        if (existingLower.endsWith('.exe')) existingChannel = 'win-exe'
        if (existingLower.endsWith('.msi')) existingChannel = 'win-msi'

        if (existingChannel === channel) {
          const oldPath = join(destDir, existing)
          console.log(`Removing old installer: ${existing}`)
          rmSync(oldPath, { force: true })
        }
      }

      cpSync(join(srcDir, file), join(destDir, file))
      copiedFiles.add(file)

      latestByChannel.set(channel, {
        file,
        channel,
        url: `https://ariatype.com/release/${file}`,
      })
      console.log(`Copied: ${file} -> public/release/`)
    }
  }
}

// Convert map values to artifacts array
for (const item of latestByChannel.values()) {
  artifacts.push(item)
}

const tauriConf = JSON.parse(
  readFileSync(join(__dirname, '../apps/desktop/src-tauri/tauri.conf.json'), 'utf8')
)
const version = tauriConf.version
const byChannel = Object.fromEntries(artifacts.map((item) => [item.channel, item.url]))

// Read existing latest.json for incremental update
const latestJsonPath = join(destDir, 'latest.json')
let existingLatest = {
  version: '',
  pub_date: '',
  notes: '',
  url: '',
  platforms: {
    mac: { universal: '', aarch64: '', x86_64: '' },
    windows: { exe: '', msi: '' },
  },
  files: [],
}

if (existsSync(latestJsonPath)) {
  try {
    existingLatest = JSON.parse(readFileSync(latestJsonPath, 'utf8'))
    console.log(`Read existing latest.json (v${existingLatest.version})`)
  } catch (err) {
    console.warn('Warning: Could not parse existing latest.json, creating new one')
  }
}

// Determine if this is a mac-aarch build (for legacy url/pub_date update)
const isMacAarchBuild = byChannel['mac-aarch'] && byChannel['mac-aarch'] !== ''

// Incremental update: merge platform info
const platforms = existingLatest.platforms || { mac: {}, windows: {} }

// Update only the platforms that were built in this run
if (byChannel['mac-universal']) platforms.mac.universal = byChannel['mac-universal']
if (byChannel['mac-aarch']) platforms.mac.aarch64 = byChannel['mac-aarch']
if (byChannel['mac-intel']) platforms.mac.x86_64 = byChannel['mac-intel']
if (byChannel['win-exe']) platforms.windows.exe = byChannel['win-exe']
if (byChannel['win-msi']) platforms.windows.msi = byChannel['win-msi']

// Merge files array: remove old files for updated channels, add new ones
const existingFiles = (existingLatest.files || []).filter(
  (f) => !latestByChannel.has(f.channel)
)
const mergedFiles = [...existingFiles, ...artifacts]

// Determine legacy fields (version, url, pub_date):
// - Only update if mac-aarch was built
// - Otherwise preserve existing values
// This ensures version consistency: version reflects the reference platform (mac-aarch)
let legacyVersion = existingLatest.version || version
let legacyUrl = existingLatest.url || ''
let pubDate = existingLatest.pub_date || ''

if (isMacAarchBuild) {
  // mac-aarch build: update all legacy fields
  legacyVersion = version
  legacyUrl = byChannel['mac-aarch']
  pubDate = new Date().toISOString()
}

const latest = {
  version: legacyVersion,
  pub_date: pubDate,
  notes: existingLatest.notes || '',
  url: legacyUrl,
  platforms,
  files: mergedFiles,
}

writeFileSync(join(destDir, 'latest.json'), JSON.stringify(latest, null, 2))
console.log(`Generated: public/release/latest.json (v${legacyVersion}) with ${copiedFiles.size} installer(s)`)
if (isMacAarchBuild) {
  console.log('  Updated legacy version/url/pub_date fields (mac-aarch build)')
}
