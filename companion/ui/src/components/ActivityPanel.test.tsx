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
});
