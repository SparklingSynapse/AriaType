import { cpSync, mkdirSync, existsSync, readdirSync, writeFileSync, readFileSync } from 'fs'
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

for (const bundleDir of bundleDirs) {
  for (const dir of installerDirs) {
    const srcDir = join(bundleDir, dir)
    if (!existsSync(srcDir)) continue

    for (const file of readdirSync(srcDir)) {
      if (!installerExts.has(extname(file))) continue
      cpSync(join(srcDir, file), join(destDir, file))
      copiedFiles.add(file)

      const lower = file.toLowerCase()
      let channel = 'unknown'
      if (lower.endsWith('.dmg')) channel = detectMacArch(file)
      if (lower.endsWith('.exe')) channel = 'win-exe'
      if (lower.endsWith('.msi')) channel = 'win-msi'

      artifacts.push({
        file,
        channel,
        url: `https://ariatype.com/release/${file}`,
      })
      console.log(`Copied: ${file} -> public/release/`)
    }
  }
}

const tauriConf = JSON.parse(
  readFileSync(join(__dirname, '../apps/desktop/src-tauri/tauri.conf.json'), 'utf8')
)
const version = tauriConf.version
const byChannel = Object.fromEntries(artifacts.map((item) => [item.channel, item.url]))
const defaultUrl =
  byChannel['mac-universal'] ||
  byChannel['mac-aarch'] ||
  byChannel['mac-intel'] ||
  byChannel['win-exe'] ||
  byChannel['win-msi'] ||
  ''

const latest = {
  version,
  pub_date: new Date().toISOString(),
  notes: '',
  url: defaultUrl,
  platforms: {
    mac: {
      universal: byChannel['mac-universal'] || '',
      aarch64: byChannel['mac-aarch'] || '',
      x86_64: byChannel['mac-intel'] || '',
    },
    windows: {
      exe: byChannel['win-exe'] || '',
      msi: byChannel['win-msi'] || '',
    },
  },
  files: artifacts,
}
writeFileSync(join(destDir, 'latest.json'), JSON.stringify(latest, null, 2))
console.log(`Generated: public/release/latest.json (v${version}) with ${copiedFiles.size} installer(s)`)
