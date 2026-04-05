/**
 * 生成 sip-cc 图标 - 纯 Node.js 无依赖
 * 输出 icon.png (256x256)，然后用 tauri icon 命令生成其他尺寸
 */
const fs = require('fs');
const zlib = require('zlib');
const path = require('path');

const SIZE = 256;

function createIcon() {
  const pixels = Buffer.alloc(SIZE * SIZE * 4);

  for (let y = 0; y < SIZE; y++) {
    for (let x = 0; x < SIZE; x++) {
      const i = (y * SIZE + x) * 4;
      const cx = SIZE / 2, cy = SIZE / 2;
      const dx = x - cx, dy = y - cy;
      const dist = Math.sqrt(dx * dx + dy * dy);
      const radius = SIZE * 0.42;
      const innerRadius = SIZE * 0.30;

      // 圆角矩形背景（模拟）
      const rx = Math.abs(dx), ry = Math.abs(dy);
      const cornerR = SIZE * 0.22;
      const halfSide = SIZE * 0.44;
      let inRect = false;
      if (rx <= halfSide && ry <= halfSide) {
        if (rx <= halfSide - cornerR || ry <= halfSide - cornerR) {
          inRect = true;
        } else {
          const cdx = rx - (halfSide - cornerR);
          const cdy = ry - (halfSide - cornerR);
          inRect = Math.sqrt(cdx * cdx + cdy * cdy) <= cornerR;
        }
      }

      if (inRect) {
        // 渐变背景：从深蓝 #1a5276 到亮蓝 #2e86c1
        const t = y / SIZE;
        const r1 = 0x1a, g1 = 0x52, b1 = 0x76;
        const r2 = 0x2e, g2 = 0x86, b2 = 0xc1;
        let r = Math.round(r1 + (r2 - r1) * t);
        let g = Math.round(g1 + (g2 - g1) * t);
        let b = Math.round(b1 + (b2 - b1) * t);

        // 画一个相机/截屏图标的简化形状
        // 相机机身（中心矩形）
        const bodyL = cx - SIZE * 0.25, bodyR = cx + SIZE * 0.25;
        const bodyT = cy - SIZE * 0.12, bodyB = cy + SIZE * 0.18;
        // 相机顶部凸起
        const topL = cx - SIZE * 0.10, topR = cx + SIZE * 0.10;
        const topT = cy - SIZE * 0.22, topB = bodyT;
        // 镜头（圆）
        const lensR = SIZE * 0.10;
        const lensDist = Math.sqrt((x - cx) * (x - cx) + (y - (cy + SIZE * 0.03)) * (y - (cy + SIZE * 0.03)));

        const inBody = x >= bodyL && x <= bodyR && y >= bodyT && y <= bodyB;
        const inTop = x >= topL && x <= topR && y >= topT && y <= topB;
        const inLens = lensDist <= lensR;
        const inLensRing = lensDist <= lensR + 3 && lensDist > lensR - 1;

        // REC 小圆点（右上角）
        const recDx = x - (cx + SIZE * 0.20), recDy = y - (cy - SIZE * 0.20);
        const recDist = Math.sqrt(recDx * recDx + recDy * recDy);
        const inRec = recDist <= SIZE * 0.06;

        if (inBody || inTop) {
          // 白色半透明相机机身
          r = 255; g = 255; b = 255;
          pixels[i + 3] = 230;
        } else if (inRec) {
          // 红色 REC 圆点
          r = 0xff; g = 0x44; b = 0x44;
          pixels[i + 3] = 255;
        } else {
          pixels[i + 3] = 255;
        }

        if (inLens) {
          // 镜头内部 - 背景色（模拟透明镜头）
          r = Math.round(r1 + (r2 - r1) * t);
          g = Math.round(g1 + (g2 - g1) * t);
          b = Math.round(b1 + (b2 - b1) * t);
          pixels[i + 3] = 255;
        }
        if (inLensRing) {
          r = 255; g = 255; b = 255;
          pixels[i + 3] = 180;
        }

        pixels[i] = r;
        pixels[i + 1] = g;
        pixels[i + 2] = b;
        if (!inBody && !inTop && !inRec) {
          pixels[i + 3] = 255;
        }
      } else {
        // 透明
        pixels[i] = 0; pixels[i + 1] = 0; pixels[i + 2] = 0; pixels[i + 3] = 0;
      }
    }
  }

  return pixels;
}

function encodePNG(width, height, pixels) {
  // 简易 PNG 编码器
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

  function chunk(type, data) {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const typeB = Buffer.from(type);
    const crcData = Buffer.concat([typeB, data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(crcData) >>> 0);
    return Buffer.concat([len, typeB, data, crc]);
  }

  // IHDR
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type: RGBA
  ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;

  // IDAT
  const rawData = Buffer.alloc(height * (1 + width * 4));
  for (let y = 0; y < height; y++) {
    rawData[y * (1 + width * 4)] = 0; // filter: none
    pixels.copy(rawData, y * (1 + width * 4) + 1, y * width * 4, (y + 1) * width * 4);
  }
  const compressed = zlib.deflateSync(rawData);

  // IEND
  const iend = Buffer.alloc(0);

  return Buffer.concat([
    signature,
    chunk('IHDR', ihdr),
    chunk('IDAT', compressed),
    chunk('IEND', iend),
  ]);
}

// CRC32 lookup table
const crcTable = new Uint32Array(256);
for (let i = 0; i < 256; i++) {
  let c = i;
  for (let j = 0; j < 8; j++) {
    c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
  }
  crcTable[i] = c;
}
function crc32(buf) {
  let crc = 0xFFFFFFFF;
  for (let i = 0; i < buf.length; i++) {
    crc = crcTable[(crc ^ buf[i]) & 0xFF] ^ (crc >>> 8);
  }
  return crc ^ 0xFFFFFFFF;
}

const pixels = createIcon();
const png = encodePNG(SIZE, SIZE, pixels);
const outPath = path.join(__dirname, '..', 'src-tauri', 'icons', 'icon-source.png');
fs.writeFileSync(outPath, png);
console.log(`Icon generated: ${outPath} (${png.length} bytes)`);
