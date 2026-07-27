import { describe, expect, it, vi } from "vitest";
import { claimPrinterInterface, sendToWebUsbPrinter } from "../webusb.js";

function fakeUsbDevice(overrides: Partial<USBDevice> = {}): USBDevice {
  const claimInterface = vi.fn().mockResolvedValue(undefined);
  const transferOut = vi.fn().mockResolvedValue({ status: "ok", bytesWritten: 2 });
  return {
    opened: true,
    configuration: {
      interfaces: [
        {
          interfaceNumber: 0,
          alternates: [{ endpoints: [{ direction: "in", type: "bulk", endpointNumber: 1 }] }],
        },
        {
          interfaceNumber: 1,
          alternates: [{ endpoints: [{ direction: "out", type: "bulk", endpointNumber: 2 }] }],
        },
      ],
    },
    open: vi.fn().mockResolvedValue(undefined),
    selectConfiguration: vi.fn().mockResolvedValue(undefined),
    claimInterface,
    transferOut,
    ...overrides,
  } as unknown as USBDevice;
}

describe("claimPrinterInterface", () => {
  it("skips interfaces with no bulk OUT endpoint and claims the first that has one", async () => {
    const device = fakeUsbDevice();
    const claimed = await claimPrinterInterface(device);
    expect(claimed).toEqual({ interfaceNumber: 1, endpointOut: 2 });
    expect(device.claimInterface).toHaveBeenCalledWith(1);
  });

  it("opens the device and selects a configuration first if not already done", async () => {
    const device = fakeUsbDevice({ opened: false, configuration: undefined });
    (device as { configuration?: unknown }).configuration = undefined;
    // After selectConfiguration "runs", pretend the config becomes available.
    (device.selectConfiguration as ReturnType<typeof vi.fn>).mockImplementation(async () => {
      (device as { configuration: USBConfiguration }).configuration = {
        interfaces: [{ interfaceNumber: 0, alternates: [{ endpoints: [{ direction: "out", type: "bulk", endpointNumber: 3 }] }] }],
      } as USBConfiguration;
    });

    await claimPrinterInterface(device);
    expect(device.open).toHaveBeenCalled();
    expect(device.selectConfiguration).toHaveBeenCalledWith(1);
  });

  it("throws when no interface exposes a bulk OUT endpoint", async () => {
    const device = fakeUsbDevice({
      configuration: {
        interfaces: [{ interfaceNumber: 0, alternates: [{ endpoints: [{ direction: "in", type: "bulk", endpointNumber: 1 }] }] }],
      } as unknown as USBConfiguration,
    });
    await expect(claimPrinterInterface(device)).rejects.toThrow(/no bulk OUT endpoint/);
  });
});

describe("sendToWebUsbPrinter", () => {
  it("transfers bytes to the claimed endpoint and succeeds on status 'ok'", async () => {
    const device = fakeUsbDevice();
    await sendToWebUsbPrinter(device, new Uint8Array([0x1b, 0x40]));
    expect(device.transferOut).toHaveBeenCalledWith(2, expect.any(Uint8Array));
  });

  it("throws when the transfer status is not 'ok'", async () => {
    const device = fakeUsbDevice({ transferOut: vi.fn().mockResolvedValue({ status: "stall" }) });
    await expect(sendToWebUsbPrinter(device, new Uint8Array([1]))).rejects.toThrow(/stall/);
  });
});
