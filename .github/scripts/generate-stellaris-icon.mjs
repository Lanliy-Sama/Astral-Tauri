import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const size = 1024;
const root = path.resolve(import.meta.dirname, '../..');
const output = path.join(root, 'src-tauri/installer/assets');
fs.mkdirSync(output, { recursive: true });

function roundedRectDistance(x, y, cx, cy, half, radius) {
  const qx = Math.abs(x - cx) - (half - radius);
  const qy = Math.abs(y - cy) - (half - radius);
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - radius;
}
function segmentDistance(px, py, x1, y1, x2, y2) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy)));
  return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
}
function coverage(distance, feather = 1.4) {
  return Math.max(0, Math.min(1, 0.5 - distance / feather));
}
function mix(a, b, t) {
  return Math.round(a + (b - a) * t);
}

const rows = [];
for (let y = 0; y < size; y++) {
  const row = Buffer.alloc(1 + size * 4);
  for (let x = 0; x < size; x++) {
    let r = 0, g = 0, b = 0, a = 0;
    const tileDistance = roundedRectDistance(x, y, 512, 512, 400, 190);
    const tileAlpha = coverage(tileDistance);
    if (tileAlpha > 0) {
      const t = (x + y) / (size * 2);
      r = mix(26, 55, t);
      g = mix(190, 108, t);
      b = mix(177, 235, t);
      a = Math.round(tileAlpha * 255);
    }
    const lineDistance = Math.min(
      segmentDistance(x, y, 512, 350, 360, 650),
      segmentDistance(x, y, 512, 350, 664, 650),
    );
    const lineAlpha = coverage(lineDistance - 18) * 0.9;
    if (lineAlpha > 0) {
      r = mix(r, 255, lineAlpha);
      g = mix(g, 255, lineAlpha);
      b = mix(b, 255, lineAlpha);
      a = Math.max(a, Math.round(lineAlpha * 255));
    }
    const nodeDistance = Math.min(
      Math.hypot(x - 512, y - 340) - 82,
      Math.hypot(x - 352, y - 650) - 72,
      Math.hypot(x - 672, y - 650) - 72,
    );
    const nodeAlpha = coverage(nodeDistance);
    if (nodeAlpha > 0) {
      r = mix(r, 255, nodeAlpha);
      g = mix(g, 255, nodeAlpha);
      b = mix(b, 255, nodeAlpha);
      a = Math.max(a, Math.round(nodeAlpha * 255));
    }
    const offset = 1 + x * 4;
    row[offset] = r;
    row[offset + 1] = g;
    row[offset + 2] = b;
    row[offset + 3] = a;
  }
  rows.push(row);
}

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const name = Buffer.from(type);
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const checksum = Buffer.alloc(4);
  checksum.writeUInt32BE(crc32(Buffer.concat([name, data])));
  return Buffer.concat([length, name, data, checksum]);
}
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(size, 0);
ihdr.writeUInt32BE(size, 4);
ihdr[8] = 8;
ihdr[9] = 6;
const png = Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', ihdr),
  chunk('IDAT', zlib.deflateSync(Buffer.concat(rows), { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);
fs.writeFileSync(path.join(output, 'stellaris-icon.png'), png);
console.log('Generated Stellaris icon:', path.join(output, 'stellaris-icon.png'));
