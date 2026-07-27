import { encode } from "fast-png";
import { describe, expect, it } from "vitest";
import { bandRaster, pngToRaster, THERMAL_58_DOTS, THERMAL_80_DOTS } from "../render/imaging.js";

function toBase64(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("base64");
}

describe("pngToRaster", () => {
  it("dithers pure black/white pixels to exact 1bpp bits with no drift", () => {
    // Gamma-correcting pure 0/255 values is a no-op (255*(v/255)^(1/gamma) is
    // 0 or 255 either way), so alternating b/w pixels accumulate zero
    // diffusion error -- lets us assert an exact expected byte.
    const width = 4;
    const height = 1;
    const data = Uint8Array.from([0, 255, 0, 255]);
    const png = encode({ width, height, data, channels: 1, depth: 8 });
    const raster = pngToRaster(toBase64(png), width);
    expect(raster.widthDots).toBe(4);
    expect(raster.heightDots).toBe(1);
    expect(raster.bytesPerRow).toBe(1);
    expect(raster.bits[0]).toBe(0xa0); // 1010_0000: bit=1 for the two black pixels
  });

  it("keeps thermal dot-width constants aligned with docs/ARCHITECTURE.md", () => {
    expect(THERMAL_58_DOTS).toBe(384);
    expect(THERMAL_80_DOTS).toBe(576);
  });

  it("downscales an oversized image to the target dot width", () => {
    const width = 800;
    const height = 100;
    const data = new Uint8Array(width * height).fill(255);
    const png = encode({ width, height, data, channels: 1, depth: 8 });
    const raster = pngToRaster(toBase64(png), 384);
    expect(raster.widthDots).toBe(384);
    expect(raster.heightDots).toBe(48); // 100 * 384/800, rounded
  });
});

describe("bandRaster", () => {
  it("splits a tall raster into row bands that reconstruct the original bits", () => {
    const width = 8;
    const height = 100;
    const data = new Uint8Array(width * height).fill(0); // all black
    const png = encode({ width, height, data, channels: 1, depth: 8 });
    const raster = pngToRaster(toBase64(png), width);
    const bands = Array.from(bandRaster(raster, 48));
    expect(bands.length).toBe(3); // 48 + 48 + 4 rows
    expect(bands.reduce((sum, b) => sum + b.rows, 0)).toBe(height);

    const reconstructed = new Uint8Array(raster.bits.length);
    let offset = 0;
    for (const band of bands) {
      reconstructed.set(band.bits, offset);
      offset += band.bits.length;
    }
    expect(reconstructed).toEqual(raster.bits);
  });
});
