#!/usr/bin/env node
/**
 * i18n Translation Checker
 * 
 * Comprehensive validation:
 * 1. Scan source code for used i18n keys (t() calls)
 * 2. Check for missing keys (used in code but not in locale files)
 * 3. Check for redundant keys (in locale files but not used in code)
 * 4. Check for empty translations
 * 
 * Run with: pnpm check:i18n
 */

import { readFileSync, readdirSync, statSync } from 'fs';
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
    if (typeof value === 'object' && value !== null) {
      getKeys(value, fullKey).forEach(k => keys.add(k));
    } else {
      keys.add(fullKey);
    }
  }
  return keys;
}

function getKeyValues(obj, prefix = '') {
  const values = {};
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null) {
      Object.assign(values, getKeyValues(value, fullKey));
    } else {
      values[fullKey] = value;
    }
  }
  return values;
}

function scanSourceFiles(srcPath, localeKeys) {
  const usedKeys = new Set();
  const returnObjectsKeys = new Set();
  const dynamicKeyWarnings = [];
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
            extractKeysFromContent(content, usedKeys, returnObjectsKeys, dynamicKeyWarnings, fullPath);
          }
        } catch {}
      }
    } catch {}
  }

  scanDir(srcPath);

  // When returnObjects: true is used with a parent key, mark all child keys as used
  for (const parentKey of returnObjectsKeys) {
    for (const key of localeKeys) {
      if (key === parentKey || key.startsWith(parentKey + '.')) {
        usedKeys.add(key);
      }
    }
  }

  return { usedKeys, dynamicKeyWarnings };
}

function extractKeysFromContent(content, usedKeys, returnObjectsKeys, dynamicKeyWarnings, filePath) {
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

  // Detect returnObjects: true usage
  const returnObjectsRegex = /t\s*\(\s*["']([^"']+)["']\s*,\s*\{[^}]*returnObjects\s*:/g;
  let match;
  while ((match = returnObjectsRegex.exec(content)) !== null) {
    returnObjectsKeys.add(match[1]);
    usedKeys.add(match[1]);
  }

  // Detect dynamic key concatenation patterns (PROHIBITED)
  // Pattern: t(`prefix.${variable}`) or t('prefix.' + variable) or t("prefix." + variable)
  const dynamicPatterns = [
    // Template literal with interpolation: t(`key.${var}`)
    /\bt\s*\(\s*`([^`]+)\$\{[^}]+\}([^`]*)`\s*[,\)]/g,
    // String concatenation: t('key.' + var) or t("key." + var)
    /\bt\s*\(\s*["']([^"']+)["']\s*\+\s*[a-zA-Z_][a-zA-Z0-9_]*\s*[,\)]/g,
  ];

  for (const regex of dynamicPatterns) {
    let m;
    while ((m = regex.exec(content)) !== null) {
      dynamicKeyWarnings.push({
        file: filePath,
        pattern: m[0].trim(),
      });
    }
  }
}

console.log('🔍 Checking i18n translations across all projects...\n');

let allPassed = true;

for (const project of PROJECTS) {
  console.log(`📦 Project: ${project.name}`);
  console.log('─'.repeat(40));
  
  const locales = {};
  for (const lang of project.languages) {
    locales[lang] = loadLocale(project.path, lang);
  }

  const enKeys = getKeys(locales.en);

  // Scan source code
  console.log('  📂 Scanning source files...');
  const { usedKeys, dynamicKeyWarnings } = scanSourceFiles(project.srcPath, enKeys);
  console.log(`  📊 Keys in code: ${usedKeys.size} | Keys in locale: ${enKeys.size}`);

  // Report dynamic key concatenation (PROHIBITED)
  if (dynamicKeyWarnings.length > 0) {
    console.log(`  🚫 ${dynamicKeyWarnings.length} DYNAMIC KEY CONCATENATION detected (PROHIBITED):`);
    dynamicKeyWarnings.forEach(w => {
      const relativePath = w.file.replace(PROJECT_ROOT, '');
      console.log(`     - ${relativePath}`);
      console.log(`       ${w.pattern}`);
    });
    allPassed = false;
  }
  
  // Missing in locale
  const missingInLocale = [...usedKeys].filter(k => !enKeys.has(k));
  if (missingInLocale.length > 0) {
    console.log(`  ❌ ${missingInLocale.length} keys USED IN CODE but MISSING in locale:`);
    missingInLocale.forEach(k => console.log(`     - ${k}`));
    allPassed = false;
  }
  
  // Redundant in locale (not used in code)
  const redundantInLocale = [...enKeys].filter(k => !usedKeys.has(k));
  
  // Known dynamic key patterns (constructed at runtime)
  // NOTE: These patterns are ONLY for keys that cannot be statically detected.
  // If code uses static keys like t("key.name"), they will be auto-detected.
  const dynamicKeyPatterns = [
    /^model\.polish\.template[A-Z]/, // templateFiller, templateFormal, templateConcise, templateAgent, templateCustom (used via switch/map)
    /^model\.domain\.subdomain_/,    // subdomain_general, subdomain_security, etc. (used via switch)
    /^dashboard\.time\./,            // time format keys (used via switch)
  ];
  
  const trulyRedundant = redundantInLocale.filter(k => 
    !dynamicKeyPatterns.some(p => p.test(k))
  );
  
  const dynamicUsed = redundantInLocale.filter(k => 
    dynamicKeyPatterns.some(p => p.test(k))
  );
  
  if (dynamicUsed.length > 0) {
    console.log(`  ℹ️  ${dynamicUsed.length} keys likely used DYNAMICALLY (excluded from redundant check):`);
    dynamicUsed.sort().forEach(k => console.log(`     - ${k}`));
  }
  
  if (trulyRedundant.length > 0) {
    console.log(`  ⚠️  ${trulyRedundant.length} keys IN LOCALE but NOT USED in code:`);
    trulyRedundant.sort().forEach(k => console.log(`     - ${k}`));
  }
  
  // Check each language
  for (const lang of project.languages) {
    if (lang === 'en') continue;
    
    const langKeys = getKeys(locales[lang]);
    const langValues = getKeyValues(locales[lang]);
    const missing = [...enKeys].filter(k => !langKeys.has(k));
    const empty = [...enKeys].filter(k => !langValues[k]);
    
    if (missing.length > 0 || empty.length > 0) {
      if (missing.length > 0) {
        console.log(`  ❌ ${lang.toUpperCase()} missing ${missing.length} keys`);
        allPassed = false;
      }
      if (empty.length > 0) {
        console.log(`  ⚠️  ${lang.toUpperCase()} has ${empty.length} empty values`);
      }
    } else {
      console.log(`  ✅ ${lang.toUpperCase()} - Complete`);
    }
  }
  console.log('');
}

console.log('═'.repeat(40));
if (allPassed) {
  console.log('✅ All i18n checks passed!');
  process.exit(0);
} else {
  console.log('❌ i18n check FAILED');
  process.exit(1);
}
