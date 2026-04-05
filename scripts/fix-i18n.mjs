import { readFileSync, writeFileSync } from 'fs';
import { join } from 'path';

const localesPath = '/Users/bytedance/git/notype/apps/desktop/src/i18n/locales';
const langs = ['ru', 'pt', 'it', 'es', 'fr', 'de', 'ko', 'ja'];
const enPath = join(localesPath, 'en.json');
const enData = JSON.parse(readFileSync(enPath, 'utf8'));

for (const lang of langs) {
  const langPath = join(localesPath, `${lang}.json`);
  const langData = JSON.parse(readFileSync(langPath, 'utf8'));
  
  let modified = false;
  for (const key of Object.keys(enData)) {
    if (!(key in langData)) {
      langData[key] = enData[key];
      modified = true;
    }
  }
  
  if (modified) {
    writeFileSync(langPath, JSON.stringify(langData, null, 2) + '\n', 'utf8');
    console.log(`Updated ${lang}.json`);
  }
}
