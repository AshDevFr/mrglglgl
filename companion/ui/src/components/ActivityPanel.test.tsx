import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Activity } from "@/bindings/Activity";
import type { Job } from "@/bindings/Job";
import { ActivityPanel } from "@/components/ActivityPanel";
import { renderWithProviders } from "@/test/render";

const api = vi.hoisted(() => ({
  now: null as Activity | null,
  activity: vi.fn(),
  cancelWriting: vi.fn(async () => true),
  cancelWaiting: vi.fn(async () => 2),
  retryWriting: vi.fn(async () => {}),
  setWritingPaused: vi.fn(async () => {}),
  cancelPreview: vi.fn(async () => true),
  cancelDownload: vi.fn(async () => {}),
  onWriting: vi.fn(async () => () => {}),
  listCharacters: vi.fn(async () => [
    {
      character: {
        id: 1,
        name: "Wren",
        surname: null,
        characterFolder: "Wren",
      },
    },
    {
      character: {
        id: 2,
        name: "Brakka",
        surname: null,
        characterFolder: "Brakka",
      },
    },
  ]),
  models: vi.fn(async () => ({
    models: [{ id: "qwen", name: "Qwen3.5 9B" }],
    chosenId: "qwen",
    advice: { kind: "recommend", id: "qwen" },
  })),
}));
vi.mock("@/api/commands", () => api);
vi.mock("@/lib/siteBuild", () => ({ useSiteBuilding: () => false }));
const navigate = vi.hoisted(() => vi.fn());
vi.mock("@tanstack/react-router", () => ({ useNavigate: () => navigate }));

function job(extra: Partial<Job>): Job {
  return {
    characterId: 1,
    kind: "day",
    startDay: "2026-11-07",
    state: "pending",
    attempts: 0,
    lastError: null,
    narrator: null,
    draft: false,
    note: null,
    partial: false,
    position: null,
    ...extra,
  };
}

function idle(): Activity {
  return {
    paused: false,
    blocked: null,
    jobs: [],
    preview: null,
    download: null,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  api.now = idle();
  api.activity.mockImplementation(async () => api.now);
});

const open = async (label: RegExp) => {
  fireEvent.click(await screen.findByRole("button", { name: label }));
  return screen.findByRole("dialog");
};

describe("ActivityPanel", () => {
  it("says when nothing is running", async () => {
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/^Activity$/);
    expect(within(drawer).getByText(/Nothing is running/)).toBeInTheDocument();
  });

  it("counts what runs and waits, for every character", async () => {
    api.now = {
      ...idle(),
      jobs: [
        job({ state: "running" }),
        job({ characterId: 2, startDay: "2026-11-06", position: 1 }),
        job({
          characterId: 1,
          kind: "week",
          startDay: "2026-11-02",
          position: 2,
        }),
        job({
          characterId: 2,
          startDay: "2026-11-01",
          state: "failed",
          lastError: "boom",
        }),
      ],
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(3\)/);
    expect(
      await within(drawer).findByText(/Wren · Saturday, November 7, 2026/),
    ).toBeInTheDocument();
    expect(
      within(drawer).getByText(/Brakka · Friday, November 6, 2026/),
    ).toBeInTheDocument();
    expect(within(drawer).getByText(/#1/)).toBeInTheDocument();
    expect(within(drawer).getByText(/#2/)).toBeInTheDocument();
    expect(
      within(drawer).getByText(/Brakka · Sunday, November 1, 2026/),
    ).toBeInTheDocument();
  });

  it("says when a waiting entry is back in line after a failed attempt", async () => {
    api.now = {
      ...idle(),
      jobs: [
        job({
          position: 1,
          attempts: 1,
          lastError: "the endpoint refused the key (unauthorised)",
        }),
        job({
          startDay: "2026-11-06",
          position: 2,
          attempts: 1,
          lastError: "timedOut",
        }),
      ],
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(2\)/);
    expect(
      await within(drawer).findByText(
        "Failed, tries again on its own in a moment: the endpoint refused the key (unauthorised)",
      ),
    ).toBeInTheDocument();
    expect(
      within(drawer).getByText(
        "Failed, tries again on its own in a moment: it took too long",
      ),
    ).toBeInTheDocument();
  });

  it("stops the entry being written and retries a failed one", async () => {
    api.now = {
      ...idle(),
      jobs: [
        job({ state: "running" }),
        job({
          characterId: 2,
          startDay: "2026-11-01",
          state: "failed",
          lastError: "boom",
        }),
      ],
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(1\)/);
    fireEvent.click(within(drawer).getByRole("button", { name: "Stop" }));
    await waitFor(() =>
      expect(api.cancelWriting).toHaveBeenCalledWith(1, "day", "2026-11-07"),
    );
    fireEvent.click(within(drawer).getByRole("button", { name: "Try again" }));
    await waitFor(() =>
      expect(api.retryWriting).toHaveBeenCalledWith(2, "day", "2026-11-01"),
    );
  });

  it("shows a preview waiting for its turn, and stops it", async () => {
    api.now = {
      ...idle(),
      jobs: [job({ state: "running" })],
      preview: { narrator: "The silly elf", running: false },
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(2\)/);
    const preview = within(drawer)
      .getByText(/Preview by The silly elf/)
      .closest("li");
    expect(preview).toHaveTextContent(/Waits for the entry being written/);
    fireEvent.click(
      within(preview as HTMLElement).getByRole("button", { name: "Stop" }),
    );
    await waitFor(() => expect(api.cancelPreview).toHaveBeenCalled());
  });

  it("shows a model download's progress, and stops it", async () => {
    api.now = {
      ...idle(),
      download: { modelId: "qwen", downloaded: 620, total: 1000 },
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(1\)/);
    const download = (await within(drawer).findByText(/Qwen3\.5 9B/)).closest(
      "li",
    );
    expect(download).toHaveTextContent("62%");
    fireEvent.click(
      within(download as HTMLElement).getByRole("button", { name: "Stop" }),
    );
    await waitFor(() => expect(api.cancelDownload).toHaveBeenCalled());
  });

  it("drops everything waiting in one go", async () => {
    api.now = {
      ...idle(),
      jobs: [
        job({ position: 1 }),
        job({ startDay: "2026-11-06", position: 2 }),
      ],
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(2\)/);
    fireEvent.click(
      await within(drawer).findByRole("button", {
        name: "Cancel the waiting ones",
      }),
    );
    await waitFor(() => expect(api.cancelWaiting).toHaveBeenCalled());
  });

  it("pauses and resumes writing", async () => {
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/^Activity$/);
    fireEvent.click(
      within(drawer).getByRole("button", { name: "Pause writing" }),
    );
    await waitFor(() =>
      expect(api.setWritingPaused).toHaveBeenCalledWith(true),
    );
  });

  it("says why nothing is being written", async () => {
    api.now = {
      ...idle(),
      paused: true,
      blocked: "paused",
      jobs: [job({ position: 1 })],
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(1\)/);
    expect(
      within(drawer).getByRole("button", { name: "Resume writing" }),
    ).toBeInTheDocument();
    expect(within(drawer).getByText(/Writing is paused/)).toBeInTheDocument();
  });

  it("warns that nothing can be written without a model, and points to it", async () => {
    api.now = {
      ...idle(),
      blocked: "noModel",
      jobs: [
        job({ position: 1 }),
        job({ startDay: "2026-11-06", position: 2 }),
      ],
    };
    renderWithProviders(<ActivityPanel />);
    // Nothing runs: the sidebar warns instead of spinning.
    const entry = await screen.findByRole("button", { name: /Activity \(2\)/ });
    expect(
      within(entry).getByLabelText("Writing is blocked"),
    ).toBeInTheDocument();
    fireEvent.click(entry);
    const drawer = await screen.findByRole("dialog");
    const warning = within(drawer).getByRole("alert");
    expect(warning).toHaveTextContent(
      "No writing model is downloaded yet. Download one in Settings, AI, under Writing models.",
    );
    fireEvent.click(
      within(warning).getByRole("button", { name: "Open Settings" }),
    );
    expect(navigate).toHaveBeenCalledWith({ to: "/settings" });
  });

  it("opens the day a task writes, or the settings a download runs in", async () => {
    api.now = {
      ...idle(),
      jobs: [job({ characterId: 2, startDay: "2026-11-06", position: 1 })],
      download: { modelId: "qwen", downloaded: 620, total: 1000 },
    };
    renderWithProviders(<ActivityPanel />);
    const drawer = await open(/Activity \(2\)/);
    fireEvent.click(
      await within(drawer).findByRole("button", {
        name: /Brakka · Friday, November 6, 2026/,
      }),
    );
    expect(navigate).toHaveBeenCalledWith({
      to: "/characters/$characterId",
      params: { characterId: "2" },
      search: { day: "2026-11-06" },
    });
    fireEvent.click(
      within(drawer).getByRole("button", { name: /Qwen3\.5 9B/ }),
    );
    expect(navigate).toHaveBeenLastCalledWith({ to: "/settings" });
  });

  it("offers the fix with every reason writing is held back", async () => {
    api.now = {
      ...idle(),
      blocked: "gameRunning",
      jobs: [job({ position: 1 })],
    };
    renderWithProviders(<ActivityPanel />);
    const warning = within(await open(/Activity \(1\)/)).getByRole("alert");
    expect(
      within(warning).getByRole("button", { name: "Open Settings" }),
    ).toBeInTheDocument();
  });

  it("resumes writing from the warning that says it is paused", async () => {
    api.now = {
      ...idle(),
      paused: true,
      blocked: "paused",
      jobs: [job({ position: 1 })],
    };
    renderWithProviders(<ActivityPanel />);
    const warning = within(await open(/Activity \(1\)/)).getByRole("alert");
    fireEvent.click(
      within(warning).getByRole("button", { name: "Resume writing" }),
    );
    await waitFor(() =>
      expect(api.setWritingPaused).toHaveBeenCalledWith(false),
    );
  });
});
