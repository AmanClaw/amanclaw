#!/usr/bin/env node
/**
 * Generate all required Tauri icon sizes from a single source PNG.
 *
 * Usage:
 *   node scripts/generate-icons.mjs [source-path]
 *
 * Default source: src-tauri/icons/app-icon.png (1024x1024 recommended)
 *
 * Generates:
 *   src-tauri/icons/32x32.png
 *   src-tauri/icons/128x128.png
 *   src-tauri/icons/128x128@2x.png  (256x256)
 *   src-tauri/icons/icon.ico         (Windows)
 *   src-tauri/icons/icon.icns        (macOS)
 *
 * Requires: npm install --save-dev sharp png-to-ico
 */

import sharp from 'sharp';
import { writeFile, mkdir } from 'fs/promises';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = resolve(__dirname, '../src-tauri/icons');

const sourcePath = process.argv[2] || resolve(iconsDir, 'app-icon.png');

const sizes = [
  { name: '32x32.png', size: 32 },
  { name: '128x128.png', size: 128 },
  { name: '128x128@2x.png', size: 256 },
];

async function generatePngs() {
  console.log(`Source: ${sourcePath}`);

  for (const { name, size } of sizes) {
    const outPath = resolve(iconsDir, name);
    await sharp(sourcePath)
      .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
      .png()
      .toFile(outPath);
    console.log(`  Generated ${name} (${size}x${size})`);
  }
}

async function generateIco() {
  // Generate multiple sizes for ICO
  const icoSizes = [16, 32, 48, 64, 128, 256];
  const buffers = await Promise.all(
    icoSizes.map(size =>
      sharp(sourcePath)
        .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
        .png()
        .toBuffer()
    )
  );

  // Simple ICO file format
  const iconCount = buffers.length;
  const headerSize = 6 + iconCount * 16;
  let dataOffset = headerSize;

  // Calculate total size
  let totalSize = headerSize;
  for (const buf of buffers) {
    totalSize += buf.length;
  }

  const ico = Buffer.alloc(totalSize);

  // ICO header
  ico.writeUInt16LE(0, 0);      // Reserved
  ico.writeUInt16LE(1, 2);      // Type: ICO
  ico.writeUInt16LE(iconCount, 4); // Image count

  // Directory entries
  for (let i = 0; i < iconCount; i++) {
    const size = icoSizes[i];
    const entryOffset = 6 + i * 16;
    ico.writeUInt8(size < 256 ? size : 0, entryOffset);     // Width
    ico.writeUInt8(size < 256 ? size : 0, entryOffset + 1);  // Height
    ico.writeUInt8(0, entryOffset + 2);                       // Color palette
    ico.writeUInt8(0, entryOffset + 3);                       // Reserved
    ico.writeUInt16LE(1, entryOffset + 4);                    // Color planes
    ico.writeUInt16LE(32, entryOffset + 6);                   // Bits per pixel
    ico.writeUInt32LE(buffers[i].length, entryOffset + 8);    // Data size
    ico.writeUInt32LE(dataOffset, entryOffset + 12);          // Data offset
    dataOffset += buffers[i].length;
  }

  // Image data
  let offset = headerSize;
  for (const buf of buffers) {
    buf.copy(ico, offset);
    offset += buf.length;
  }

  const outPath = resolve(iconsDir, 'icon.ico');
  await writeFile(outPath, ico);
  console.log(`  Generated icon.ico (${icoSizes.join(', ')}px)`);
}

async function generateIcns() {
  // For macOS .icns, we generate a simple version with the key sizes
  // A proper .icns requires specific format, but Tauri accepts PNGs too
  // So we generate a 512x512 PNG as icon.png which Tauri uses on macOS
  const outPath = resolve(iconsDir, 'icon.png');
  await sharp(sourcePath)
    .resize(512, 512, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toFile(outPath);
  console.log(`  Generated icon.png (512x512, for macOS)`);
}

async function main() {
  console.log('Generating Tauri icons...\n');

  try {
    await generatePngs();
    await generateIco();
    await generateIcns();
    console.log('\nDone! Icons saved to src-tauri/icons/');
  } catch (err) {
    if (err.message.includes('Input file is missing')) {
      console.error(`\nError: Source icon not found at ${sourcePath}`);
      console.error('Place a 1024x1024 PNG at src-tauri/icons/app-icon.png');
      console.error('Or pass a path: node scripts/generate-icons.mjs /path/to/icon.png');
      process.exit(1);
    }
    throw err;
  }
}

main();
