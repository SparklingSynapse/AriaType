import { cpSync, mkdirSync, existsSync, readdirSync, writeFileSync, readFileSync } from 'fs'
import { join, extname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))
const targetBase = join(__dirname, '../apps/desktop/src-tauri/target')
// universal build outputs to universal-apple-darwin, fallback to release (arm/intel or windows)
const bundleDir =
  existsSync(join(targetBase, 'universal-apple-darwin/release/bundle'))
    ? join(targetBase, 'universal-apple-darwin/release/bundle')
    : join(targetBase, 'release/bundle')
const destDir = join(__dirname, '../packages/website/public/release')

mkdirSync(destDir, { recursive: true })

if (!existsSync(bundleDir)) {
  console.log('No bundle directory found, skipping installer copy.')
  process.exit(0)
}

const installerExts = new Set(['.dmg', '.exe', '.msi'])
const bundleDirs = ['dmg', 'nsis', 'msi']
let dmgName = null

for (const dir of bundleDirs) {
  const srcDir = join(bundleDir, dir)
  if (!existsSync(srcDir)) continue

  for (const file of readdirSync(srcDir)) {
    if (installerExts.has(extname(file))) {
      cpSync(join(srcDir, file), join(destDir, file))
      console.log(`Copied: ${file} -> public/release/`)
      if (file.endsWith('.dmg')) dmgName = file
    }
  }
}

// Generate release/latest.json (same directory as installers)
const tauriConf = JSON.parse(
  readFileSync(join(__dirname, '../apps/desktop/src-tauri/tauri.conf.json'), 'utf8')
)
const version = tauriConf.version

const latest = {
  version,
  pub_date: new Date().toISOString(),
  notes: '',
  url: dmgName ? `https://ariatype.com/release/${dmgName}` : '',
}
writeFileSync(join(destDir, 'latest.json'), JSON.stringify(latest, null, 2))
console.log(`Generated: public/release/latest.json (v${version})`)
