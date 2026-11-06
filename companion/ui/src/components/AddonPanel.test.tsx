import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AddonView } from "@/bindings/AddonView";
import type { FlavorAddon } from "@/bindings/FlavorAddon";
import type { ReleaseCheck } from "@/bindings/ReleaseCheck";
import { AddonPanel } from "@/components/AddonPanel";
import { renderWithProviders } from "@/test/render";

const FLAVOR = "/Games/World of Warcraft/_forever_";

const released = {
  version: "0.2.0",
  interface: 16001,
  schema: 1,
  development: false,
};

const api = vi.hoisted(() => ({
  views: [] as AddonView[],
  check: null as ReleaseCheck | null,
  addonViews: vi.fn(),
  addonRelease: vi.fn(),
  setAddonPrereleases: vi.fn(async () => {}),
  installAddon: vi.fn(async () => ({ version: "0.3.0", replaced: true })),
  removeAddon: vi.fn(async () => {}),
  restoreAddon: vi.fn(async () => ({ version: "0.1.0", replaced: true })),
}));
vi.mock("@/api/commands", () => api);

function status(extra: Partial<FlavorAddon>): FlavorAddon {
  return {
    flavor: "_forever_",
    folder: `${FLAVOR}/Interface/AddOns/Mrglglgl`,
    clientVersion: "1.60.1.70790",
    installed: released,
    lastRun: null,
    state: "waiting",
    problems: [],
    ...extra,
  };
}

function view(extra: Partial<AddonView>, s: Partial<FlavorAddon> = {}) {
  return {
    status: status(s),
    flavorPath: FLAVOR,
    ownership: "managed",
    rollback: null,
    ...extra,
  } as AddonView;
}

function release(updates: string[] = []): ReleaseCheck {
  return {
    release: {
      tag: "addon-v0.3.0",
      version: "0.3.0",
      prerelease: false,
      url: "https://github.com/AshDevFr/mrglglgl/releases/download/addon-v0.3.0/x-forever.zip",
      size: 27000,
      sha256: "ab",
      published: null,
    },
    prereleases: false,
    updates,
    checked: 1794000000,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  api.addonViews.mockImplementation(async () => api.views);
  api.addonRelease.mockImplementation(async () => api.check);
  api.check = release();
});

describe("AddonPanel", () => {
  it("offers to install the newest release where the addon is missing", async () => {
    api.views = [
      view(
        { ownership: "notInstalled" },
        { installed: null, state: "notInstalled" },
      ),
    ];
    renderWithProviders(<AddonPanel />);
    expect(await screen.findByText("Not installed")).toBeInTheDocument();
    expect(
      screen.getByText(/is not in \/Games\/World of Warcraft\/_forever_/),
    ).toBeInTheDocument();
    expect(screen.getByText("Game version 1.60.1.70790")).toBeInTheDocument();
    expect(
      await screen.findByText(/Newest release: 0\.3\.0/),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByText("Install version 0.3.0"));
    await waitFor(() =>
      expect(api.installAddon).toHaveBeenCalledWith(FLAVOR, false),
    );
    expect(
      await screen.findByText(/Version 0\.3\.0 is installed\. Type \/reload/),
    ).toBeInTheDocument();
  });

  it("offers an update only where an older copy the app installed is found", async () => {
    api.views = [
      view(
        { ownership: "managed" },
        {
          state: "recording",
          lastRun: {
            character: "Wren",
            version: "0.2.0",
            interface: 16001,
            login: 1794000000,
          },
        },
      ),
    ];
    api.check = release([FLAVOR]);
    renderWithProviders(<AddonPanel />);
    expect(await screen.findByText("Update to 0.3.0")).toBeInTheDocument();
    expect(screen.getByText("Version 0.2.0")).toBeInTheDocument();
    expect(screen.getByText(/Last noted by Wren on/)).toBeInTheDocument();
  });

  it("leaves a copy installed by something else alone unless the player takes it over", async () => {
    api.views = [view({ ownership: "foreign" })];
    renderWithProviders(<AddonPanel />);
    expect(
      await screen.findByText(/Installed by something else/),
    ).toBeInTheDocument();
    expect(screen.queryByText("Remove the addon")).toBeNull();
    fireEvent.click(
      await screen.findByText("Replace with version 0.3.0 and look after it"),
    );
    await waitFor(() =>
      expect(api.installAddon).toHaveBeenCalledWith(FLAVOR, true),
    );
  });

  it("asks before removing, and says recorded journals stay", async () => {
    api.views = [view({ ownership: "managed" })];
    renderWithProviders(<AddonPanel />);
    fireEvent.click(await screen.findByText("Remove the addon"));
    expect(
      screen.getByText(
        /Everything it recorded stays: in the game's WTF folder/,
      ),
    ).toBeInTheDocument();
    expect(api.removeAddon).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("Remove it"));
    await waitFor(() =>
      expect(api.removeAddon).toHaveBeenCalledWith(FLAVOR, false),
    );
    expect(
      await screen.findByText(
        /The addon was removed\. Your journals are untouched/,
      ),
    ).toBeInTheDocument();
  });

  it("puts back a kept copy", async () => {
    api.views = [
      view(
        {
          ownership: "notInstalled",
          rollback: {
            path: "/data/addon/forever/rollback/1-0.1.0",
            version: "0.1.0",
            taken: 1,
          },
        },
        { installed: null, state: "notInstalled" },
      ),
    ];
    renderWithProviders(<AddonPanel />);
    fireEvent.click(await screen.findByText("Put back version 0.1.0"));
    await waitFor(() => expect(api.restoreAddon).toHaveBeenCalledWith(FLAVOR));
  });

  it("never offers to change a development copy", async () => {
    api.views = [
      view(
        { ownership: "development" },
        { installed: { ...released, development: true, version: null } },
      ),
    ];
    renderWithProviders(<AddonPanel />);
    expect(await screen.findByText("Development copy")).toBeInTheDocument();
    expect(screen.getByText(/never writes to it/)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Install|Update|Remove|Replace/ }),
    ).toBeNull();
  });

  it("shows a refused change with what to do next", async () => {
    api.views = [
      view(
        { ownership: "notInstalled" },
        { installed: null, state: "notInstalled" },
      ),
    ];
    api.installAddon.mockRejectedValueOnce("the game is running");
    renderWithProviders(<AddonPanel />);
    fireEvent.click(await screen.findByText("Install version 0.3.0"));
    expect(
      await screen.findByText(
        /The addon was not changed: the game is running\. The game folder was left as it was/,
      ),
    ).toBeInTheDocument();
  });

  it("shows each problem with what to do about it", async () => {
    api.views = [
      view(
        {},
        {
          state: "recording",
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
        },
      ),
    ];
    renderWithProviders(<AddonPanel />);
    expect(
      await screen.findByText(
        /still running version 0\.1\.0; version 0\.2\.0 is installed\. Type \/reload/,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Loot\.lua:40: oops\. Recording carried on/),
    ).toBeInTheDocument();
  });

  it("keeps working when the releases cannot be looked up", async () => {
    api.views = [view({ ownership: "managed" })];
    api.addonRelease.mockRejectedValue("offline");
    renderWithProviders(<AddonPanel />);
    expect(
      await screen.findByText(
        /Could not look up the addon's releases: offline/,
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Remove the addon")).toBeInTheDocument();
  });

  it("turns test versions on", async () => {
    api.views = [];
    renderWithProviders(<AddonPanel />);
    expect(
      await screen.findByText(/No WoW: Forever folder found/),
    ).toBeInTheDocument();
    const toggle = await screen.findByRole("switch");
    await waitFor(() => expect(toggle).not.toBeDisabled());
    fireEvent.click(toggle);
    await waitFor(() =>
      expect(api.setAddonPrereleases).toHaveBeenCalledWith(true),
    );
  });
});
