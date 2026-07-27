import { describe, expect, it } from "vitest";
import { chunkBytes } from "../web-bluetooth.js";

describe("chunkBytes", () => {
  it("splits into chunks no larger than chunkSize", () => {
    const bytes = new Uint8Array(45).map((_, i) => i);
    const chunks = chunkBytes(bytes, 20);
    expect(chunks).toHaveLength(3);
    expect(chunks[0]).toHaveLength(20);
    expect(chunks[1]).toHaveLength(20);
    expect(chunks[2]).toHaveLength(5);
  });

  it("reassembles to the original bytes", () => {
    const bytes = new Uint8Array(45).map((_, i) => i);
    const chunks = chunkBytes(bytes, 20);
    const reassembled = new Uint8Array(bytes.length);
    let offset = 0;
    for (const c of chunks) {
      reassembled.set(c, offset);
      offset += c.length;
    }
    expect(reassembled).toEqual(bytes);
  });

  it("returns a single chunk when bytes fit within chunkSize", () => {
    const bytes = new Uint8Array([1, 2, 3]);
    expect(chunkBytes(bytes, 20)).toEqual([bytes]);
  });

  it("returns an empty array for empty input", () => {
    expect(chunkBytes(new Uint8Array(0), 20)).toEqual([]);
  });
});
