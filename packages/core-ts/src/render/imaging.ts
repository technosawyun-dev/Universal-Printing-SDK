// PNG -> 1bpp dithered raster, for the ESC/POS `GS v 0` raster image command.
//
// escpos-php's GdEscposImage (see "Opensource Printers printing/escpos-php")
// converts to black/white with a flat >>7 brightness threshold — no
// dithering at all, which is why images printed through it look blotchy on
// anything but line art. We gamma-correct then Floyd-Steinberg dither
// instead, matching the approach documented in docs/ARCHITECTURE.md.

import { decode } from "fast-png";

export const THERMAL_58_DOTS = 384;
export const THERMAL_80_DOTS = 576;
const DEFAULT_GAMMA = 1.8;

export interface RasterImage {
  widthDots: number;
  heightDots: number;
  bytesPerRow: number;
  /** 1 bit per pixel, MSB-first, row-major; bit=1 means "print a dot" (black). */
  bits: Uint8Array;
}

export function base64ToBytes(base64: string): Uint8Array {
  const globalAtob = (globalThis as { atob?: (s: string) => string }).atob;
  if (typeof globalAtob === "function") {
    const bin = globalAtob(base64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }
  const nodeBuffer = (globalThis as { Buffer?: { from(s: string, enc: string): Uint8Array } }).Buffer;
  if (nodeBuffer) return new Uint8Array(nodeBuffer.from(base64, "base64"));
  throw new Error("No base64 decoder available in this environment (need atob or Buffer)");
}

/** `scale` normalizes 16-bit-depth PNG samples (0-65535) down to the 0-255 range this module works in. */
function toGrayscale(data: ArrayLike<number>, width: number, height: number, channels: number, scale: number): Float64Array {
  const gray = new Float64Array(width * height);
  for (let i = 0, p = 0; i < width * height; i++, p += channels) {
    if (channels === 1 || channels === 2) {
      gray[i] = data[p] * scale;
    } else {
      gray[i] = (0.299 * data[p] + 0.587 * data[p + 1] + 0.114 * data[p + 2]) * scale;
    }
  }
  return gray;
}

/** Nearest-neighbor resize to `targetWidth`, preserving aspect ratio. */
function resize(
  gray: Float64Array,
  srcWidth: number,
  srcHeight: number,
  targetWidth: number,
): { data: Float64Array; width: number; height: number } {
  if (srcWidth === targetWidth) {
    return { data: gray, width: srcWidth, height: srcHeight };
  }
  const targetHeight = Math.max(1, Math.round((srcHeight * targetWidth) / srcWidth));
  const out = new Float64Array(targetWidth * targetHeight);
  for (let y = 0; y < targetHeight; y++) {
    const sy = Math.min(srcHeight - 1, Math.floor((y * srcHeight) / targetHeight));
    for (let x = 0; x < targetWidth; x++) {
      const sx = Math.min(srcWidth - 1, Math.floor((x * srcWidth) / targetWidth));
      out[y * targetWidth + x] = gray[sy * srcWidth + sx];
    }
  }
  return { data: out, width: targetWidth, height: targetHeight };
}

function applyGamma(gray: Float64Array, gamma: number): Float64Array {
  const out = new Float64Array(gray.length);
  for (let i = 0; i < gray.length; i++) {
    out[i] = 255 * Math.pow(gray[i] / 255, 1 / gamma);
  }
  return out;
}

/** In-place Floyd-Steinberg error-diffusion dither; returns 1bpp packed bits (bit=1 -> black). */
function ditherToBits(gray: Float64Array, width: number, height: number): Uint8Array {
  const buf = Float64Array.from(gray);
  const bytesPerRow = Math.ceil(width / 8);
  const bits = new Uint8Array(bytesPerRow * height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const idx = y * width + x;
      const old = buf[idx];
      const black = old < 128;
      const newVal = black ? 0 : 255;
      const err = old - newVal;
      if (black) bits[y * bytesPerRow + (x >> 3)] |= 0x80 >> (x & 7);
      if (x + 1 < width) buf[idx + 1] += (err * 7) / 16;
      if (y + 1 < height) {
        if (x - 1 >= 0) buf[idx + width - 1] += (err * 3) / 16;
        buf[idx + width] += (err * 5) / 16;
        if (x + 1 < width) buf[idx + width + 1] += (err * 1) / 16;
      }
    }
  }
  return bits;
}

/** Decodes a base64 PNG and dithers it to a 1bpp raster no wider than `targetWidthDots`. */
export function pngToRaster(
  base64Png: string,
  targetWidthDots: number,
  gamma: number = DEFAULT_GAMMA,
): RasterImage {
  const png = decode(base64ToBytes(base64Png));
  const scale = png.depth === 16 ? 255 / 65535 : 1;
  const gray = toGrayscale(png.data, png.width, png.height, png.channels, scale);
  const clampedWidth = Math.min(targetWidthDots, png.width);
  const resized = resize(gray, png.width, png.height, clampedWidth);
  const gammaCorrected = applyGamma(resized.data, gamma);
  const bits = ditherToBits(gammaCorrected, resized.width, resized.height);
  return {
    widthDots: resized.width,
    heightDots: resized.height,
    bytesPerRow: Math.ceil(resized.width / 8),
    bits,
  };
}

/** Splits a raster into row-bands so `GS v 0` chunks stay small enough for the printer's receive buffer. */
export function* bandRaster(image: RasterImage, bandRows = 48): Generator<{ rows: number; bits: Uint8Array }> {
  for (let y = 0; y < image.heightDots; y += bandRows) {
    const rows = Math.min(bandRows, image.heightDots - y);
    const start = y * image.bytesPerRow;
    const end = start + rows * image.bytesPerRow;
    yield { rows, bits: image.bits.subarray(start, end) };
  }
}
