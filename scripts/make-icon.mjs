// Génère l'icône source (PNG 1024x1024) sans dépendance externe.
import zlib from "node:zlib";
import fs from "node:fs";
const S = 1024;
const buf = Buffer.alloc(S * (S * 4 + 1));
const inRound = (x, y, r, m) => {
  const dx = Math.max(m + r - x, 0, x - (S - m - r)), dy = Math.max(m + r - y, 0, y - (S - m - r));
  return x >= m && x < S - m && y >= m && y < S - m && dx * dx + dy * dy <= r * r;
};
const inTri = (x, y) => {
  // triangle "play" centré
  const ax = 400, ay = 300, bx = 400, by = 724, cx = 760, cy = 512;
  const s = (x1, y1, x2, y2) => (x - x2) * (y1 - y2) - (x1 - x2) * (y - y2);
  const d1 = s(ax, ay, bx, by), d2 = s(bx, by, cx, cy), d3 = s(cx, cy, ax, ay);
  return !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0));
};
for (let y = 0; y < S; y++) {
  const row = y * (S * 4 + 1);
  buf[row] = 0;
  for (let x = 0; x < S; x++) {
    const o = row + 1 + x * 4;
    if (!inRound(x, y, 220, 40)) { buf[o + 3] = 0; continue; }
    const t = (x + y) / (2 * S);
    let r = 99 + (168 - 99) * t, g = 102 + (85 - 102) * t, b = 241 + (247 - 241) * t;
    if (inTri(x, y)) { r = g = b = 255; }
    buf[o] = r; buf[o + 1] = g; buf[o + 2] = b; buf[o + 3] = 255;
  }
}
const crcT = [...Array(256)].map((_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc = (b) => { let c = 0xffffffff; for (const v of b) c = crcT[(c ^ v) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
const chunk = (type, data) => { const l = Buffer.alloc(4); l.writeUInt32BE(data.length); const td = Buffer.concat([Buffer.from(type), data]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([l, td, c]); };
const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(S, 0); ihdr.writeUInt32BE(S, 4); ihdr[8] = 8; ihdr[9] = 6;
fs.writeFileSync(process.argv[2], Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", zlib.deflateSync(buf)), chunk("IEND", Buffer.alloc(0))]));
