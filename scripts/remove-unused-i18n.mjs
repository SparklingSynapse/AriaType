#!/usr/bin/env node
/**
 * Remove unused i18n keys from locale files
 * 
 * Run with: node scripts/remove-unused-i18n.mjs
 */

import { readFileSync, readdirSync, statSync, writeFileSync } from 'fs';
import { join, dirname, extname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = join(__dirname, '..');

const PROJECTS = [
  {
    name: 'desktop',
    path: join(PROJECT_ROOT, 'apps/desktop/src/i18n/locales'),
    srcPath: join(PROJECT_ROOT, 'apps/desktop/src'),
    languages: ['en', 'zh', 'ru', 'pt', 'it', 'es', 'fr', 'de', 'ko', 'ja']
  },
  {
    name: 'website',
    path: join(PROJECT_ROOT, 'packages/website/src/i18n/locales'),
    srcPath: join(PROJECT_ROOT, 'packages/website/src'),
    languages: ['en', 'zh']
  }
];

function loadLocale(localesPath, lang) {
  const filePath = join(localesPath, `${lang}.json`);
  try {
    const content = readFileSync(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (e) {
    console.error(`Failed to load ${lang}.json: ${e.message}`);
    process.exit(1);
  }
}

function getKeys(obj, prefix = '') {
  const keys = new Set();
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      getKeys(value, fullKey).forEach(k => keys.add(k));
    } else {
      keys.add(fullKey);
    }
  }
  return keys;
}

function scanSourceFiles(srcPath) {
  const usedKeys = new Set();
  const returnObjectsKeys = new Set();
  const extensions = ['.tsx', '.ts', '.jsx', '.js'];
  
  function scanDir(dir) {
    try {
      const entries = readdirSync(dir);
      for (const entry of entries) {
        const fullPath = join(dir, entry);
        try {
          const stat = statSync(fullPath);
          if (stat.isDirectory()) {
            if (entry === 'node_modules' || entry === 'dist' || entry === 'build') continue;
            scanDir(fullPath);
          } else if (stat.isFile() && extensions.includes(extname(entry))) {
            const content = readFileSync(fullPath, 'utf-8');
            extractKeysFromContent(content, usedKeys, returnObjectsKeys);
          }
        } catch {}
      }
    } catch {}
  }
  
  scanDir(srcPath);
  return { usedKeys, returnObjectsKeys };
}

function extractKeysFromContent(content, usedKeys, returnObjectsKeys) {
  const patterns = [
    /\bt\s*\(\s*"([^"]+)"\s*[,\)]/g,
    /\bt\s*\(\s*'([^']+)'\s*[,\)]/g,
  ];
  
  for (const regex of patterns) {
    let match;
    while ((match = regex.exec(content)) !== null) {
      usedKeys.add(match[1]);
    }
  }
  
  const returnObjectsRegex = /t\s*\(\s*["']([^"']+)["']\s*,\s*\{[^}]*returnObjects\s*:/g;
  let match;
  while ((match = returnObjectsRegex.exec(content)) !== null) {
    returnObjectsKeys.add(match[1]);
    usedKeys.add(match[1]);
  }
}

function removeUnusedKeys(obj, usedKeys, returnObjectsKeys, prefix = '') {
  const result = {};
  
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      // Check if this is a parent key used with returnObjects
      if (returnObjectsKeys.has(fullKey)) {
        result[key] = value;
      } else {
        const cleaned = removeUnusedKeys(value, usedKeys, returnObjectsKeys, fullKey);
        if (Object.keys(cleaned).length > 0) {
          result[key] = cleaned;
        }
      }
    } else {
      if (usedKeys.has(fullKey)) {
        result[key] = value;
      }
    }
  }
  
  return result;
}

console.log('🧹 Removing unused i18n keys...\n');

for (const project of PROJECTS) {
  console.log(`📦 Project: ${project.name}`);
  console.log('─'.repeat(40));
  
  const { usedKeys, returnObjectsKeys } = scanSourceFiles(project.srcPath);
  
  for (const lang of project.languages) {
    const locale = loadLocale(project.path, lang);
    const originalKeys = getKeys(locale);
    
    const cleaned = removeUnusedKeys(locale, usedKeys, returnObjectsKeys);
    const cleanedKeys = getKeys(cleaned);
    
    const removed = originalKeys.size - cleanedKeys.size;
    
    if (removed > 0) {
      const filePath = join(project.path, `${lang}.json`);
      writeFileSync(filePath, JSON.stringify(cleaned, null, 2) + '\n');
      console.log(`  ✅ ${lang.toUpperCase()}: Removed ${removed} unused keys (${originalKeys.size} → ${cleanedKeys.size})`);
    } else {
      console.log(`  ✅ ${lang.toUpperCase()}: No unused keys`);
    }
  }
  
  console.log('');
}

console.log('═'.repeat(40));
console.log('✅ Done! Run pnpm check:i18n to verify.');