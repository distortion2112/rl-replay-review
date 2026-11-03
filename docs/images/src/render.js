// Renders each <section class="shot"> in readme-graphics.html to docs/images/<id>.png.
// Usage (from the repo root): node docs/images/src/render.js
// Needs Playwright with Chromium and the Inter / Inter Display fonts installed.
// Playwright is not a project dependency; install it globally or with npx.
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const here = path.dirname(fileURLToPath(import.meta.url));
const src = path.join(here, 'readme-graphics.html');
const outDir = path.join(here, '..');

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1.25 });
await page.goto('file://' + src);
await page.evaluate(() => document.fonts.ready);
for (const el of await page.$$('section.shot')) {
  const id = await el.getAttribute('id');
  const out = path.join(outDir, `${id}.png`);
  await el.screenshot({ path: out });
  console.log('wrote', path.relative(process.cwd(), out));
}
await browser.close();
