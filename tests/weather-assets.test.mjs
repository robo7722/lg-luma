import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import test from 'node:test';
import sharp from 'sharp';
import { names } from '../tools/weather-icons.mjs';

const directory = new URL('../assets/weather/', import.meta.url);

test('original weather icons have SVG sources, pinned PNGs, colour and transparency', async () => {
  const hashes = JSON.parse(await fs.readFile(new URL('checksums.json', directory), 'utf8'));
  assert.deepEqual(Object.keys(hashes).sort(), names.map((name) => `${name}.png`).sort());
  for (const name of names) {
    const svg = await fs.readFile(new URL(`${name}.svg`, directory), 'utf8');
    assert.match(svg, /Original Luma weather artwork, MIT/);
    assert.doesNotMatch(svg, /<image\b|<text\b|href=/, name);
    const file = `${name}.png`;
    const data = await fs.readFile(new URL(file, directory));
    assert.equal(createHash('sha256').update(data).digest('hex'), hashes[file], file);
    const metadata = await sharp(data).metadata();
    assert.equal(metadata.format, 'png');
    assert.equal(metadata.width, 256, file);
    assert.equal(metadata.height, 256, file);
    assert.equal(metadata.hasAlpha, true, file);
    const stats = await sharp(data).stats();
    assert.equal(stats.channels[3].min, 0, file);
    assert.ok(
      stats.channels.slice(0, 3).some((channel) => channel.stdev > 10),
      file,
    );
  }
});
