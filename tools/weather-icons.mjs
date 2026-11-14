// Renders the original weather SVGs in assets/weather/ to the 256 px PNGs the launcher loads,
// and pins their hashes. Run after editing an SVG: node tools/weather-icons.mjs
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import sharp from 'sharp';

const directory = path.resolve(import.meta.dirname, '../assets/weather');
export const names = [
  'sun',
  'moon',
  'partly-cloudy',
  'partly-cloudy-night',
  'cloud',
  'fog',
  'drizzle',
  'rain',
  'snow',
  'storm',
];

export async function renderWeatherIcon(name) {
  const svg = await fs.readFile(path.join(directory, `${name}.svg`));
  return sharp(svg, { density: 144 }).resize(256, 256).png({ compressionLevel: 9 }).toBuffer();
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const hashes = {};
  for (const name of names) {
    const png = await renderWeatherIcon(name);
    await fs.writeFile(path.join(directory, `${name}.png`), png);
    hashes[`${name}.png`] = createHash('sha256').update(png).digest('hex');
  }
  await fs.writeFile(
    path.join(directory, 'checksums.json'),
    `${JSON.stringify(hashes, null, 2)}\n`,
  );
  console.log(`Rendered ${names.length} weather icons`);
}
