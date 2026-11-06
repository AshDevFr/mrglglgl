import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { FlavorAddon } from "@/bindings/FlavorAddon";
import { AddonPanel } from "@/components/AddonPanel";
import { renderWithProviders } from "@/test/render";

const released = {
  version: "0.2.0",
  interface: 16001,
  schema: 1,
  development: false,
};

const status = vi.hoisted(() => ({ flavors: [] as FlavorAddon[] }));
vi.mock("@/api/commands", () => ({
  addonStatus: async () => status.flavors,
}));

function flavor(extra: Partial<FlavorAddon>): FlavorAddon {
  return {
    flavor: "_forever_",
    folder: "/Games/World of Warcraft/_forever_/Interface/AddOns/Mrglglgl",
    clientVersion: "1.60.1.70790",
    installed: released,
    lastRun: null,
    state: "waiting",
    problems: [],
    ...extra,
  };
}

describe("AddonPanel", () => {
  it("says where to install the addon when it is missing", async () => {
    status.flavors = [flavor({ installed: null, state: "notInstalled" })];
    renderWithProviders(<AddonPanel />);
    expect(await screen.findByText("Not installed")).toBeInTheDocument();
    expect(
      screen.getByText(/is not in \/Games\/World of Warcraft\/_forever_/),
    ).toBeInTheDocument();
    expect(screen.getByText("Game version 1.60.1.70790")).toBeInTheDocument();
  });

  it("shows the version and who last recorded", async () => {
    status.flavors = [
      flavor({
        state: "recording",
        lastRun: {
          character: "Wren",
          version: "0.2.0",
          interface: 16001,
          login: 1794000000,
        },
      }),
    ];
    renderWithProviders(<AddonPanel />);
    expect(await screen.findByText("Recording")).toBeInTheDocument();
    expect(screen.getByText("Version 0.2.0")).toBeInTheDocument();
    expect(screen.getByText(/Last noted by Wren on/)).toBeInTheDocument();
  });

  it("shows each problem with what to do about it", async () => {
    status.flavors = [
      flavor({
        state: "recording",
        installed: { ...released, development: true, version: null },
        lastRun: {
          character: "Wren",
          version: "0.1.0",
          interface: 16002,
          login: 1794000000,
        },
        problems: [
          { kind: "updatePending", running: "0.1.0", installed: "0.2.0" },
          {
            kind: "errors",
            count: 2,
            message: "Loot.lua:40: oops",
            time: 1794000000,
          },
        ],
      }),
    ];
    renderWithProviders(<AddonPanel />);
    expect(await screen.findByText("Development copy")).toBeInTheDocument();
    expect(
      screen.getByText(
        /still running version 0\.1\.0; version 0\.2\.0 is installed\. Type \/reload/,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Loot\.lua:40: oops\. Recording carried on/),
    ).toBeInTheDocument();
  });

  it("says when no Forever folder is found", async () => {
    status.flavors = [];
    renderWithProviders(<AddonPanel />);
    expect(
      await screen.findByText(/No WoW: Forever folder found/),
    ).toBeInTheDocument();
  });
});
