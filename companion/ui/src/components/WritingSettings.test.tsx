import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelsView } from "@/bindings/ModelsView";
import type { WritingSettings } from "@/bindings/WritingSettings";
import { defaultWriting, renderWithProviders } from "@/test/render";
import { EndpointFields } from "./EndpointFields";
import { ModelsPanel } from "./ModelsPanel";

const commands = vi.hoisted(() => ({
  endpointSendsOffMachine: vi.fn(
    async (url: string) => !url.includes("localhost"),
  ),
  hasApiKey: vi.fn(async () => false),
  saveApiKey: vi.fn(async () => {}),
  testEndpoint: vi.fn(async () => {}),
  models: vi.fn(),
  downloadModel: vi.fn(async () => {}),
  cancelDownload: vi.fn(async () => {}),
  deleteModel: vi.fn(async () => {}),
  onModelDownload: vi.fn(async () => () => {}),
}));
vi.mock("@/api/commands", () => commands);

function endpoint(extra: Partial<WritingSettings>) {
  const onChange = vi.fn();
  renderWithProviders(
    <EndpointFields
      value={{
        ...defaultWriting,
        provider: "endpoint",
        endpointModel: "scribe",
        ...extra,
      }}
      onChange={onChange}
    />,
  );
  return onChange;
}

describe("EndpointFields", () => {
  it("states plainly what is sent to an endpoint that is not on this computer", async () => {
    endpoint({ endpointUrl: "https://api.example.com/v1" });
    expect(
      await screen.findByText(
        /This address is not on this computer\. When writing, the app sends api\.example\.com: your character's name, the names of people you grouped with, and the text of quests/,
      ),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/Send stand-in names/)).toBeInTheDocument();
  });

  it("says nothing of the kind for an endpoint on this computer", async () => {
    endpoint({ endpointUrl: "http://localhost:11434/v1" });
    await waitFor(() =>
      expect(commands.endpointSendsOffMachine).toHaveBeenCalledWith(
        "http://localhost:11434/v1",
      ),
    );
    expect(screen.queryByText(/not on this computer/)).not.toBeInTheDocument();
  });

  it("tests the connection and reports the answer", async () => {
    commands.testEndpoint.mockRejectedValueOnce(
      "the endpoint refused the key (unauthorised)",
    );
    endpoint({ endpointUrl: "http://localhost:11434/v1" });
    fireEvent.click(screen.getByRole("button", { name: "Test connection" }));
    expect(
      await screen.findByText(
        /^Test failed: the endpoint refused the key \(unauthorised\)\. Check the address/,
      ),
    ).toBeInTheDocument();
  });
});

const view: ModelsView = {
  chosenId: "qwen3.5-4b",
  advice: { kind: "recommend", id: "qwen3.5-4b" },
  models: [
    {
      id: "qwen3.5-4b",
      name: "Qwen3.5 4B",
      sizeBytes: 2_740_937_888,
      licence: "Apache-2.0",
      languages: ["en", "fr"],
      minRamGb: 8,
      recommended: true,
      proseNote: "Best writing of the models tested.",
      installed: false,
      partialBytes: 0,
    },
    {
      id: "smollm3-3b",
      name: "SmolLM3 3B",
      sizeBytes: 1_915_306_528,
      licence: "Apache-2.0",
      languages: ["en", "fr"],
      minRamGb: 8,
      recommended: false,
      proseNote: null,
      installed: true,
      partialBytes: 0,
    },
  ],
};

describe("ModelsPanel", () => {
  beforeEach(() => {
    commands.models.mockReset();
    commands.downloadModel.mockClear();
  });

  it("downloads only after the size is confirmed", async () => {
    commands.models.mockResolvedValue({
      ...view,
      models: view.models.map((m) => ({ ...m, installed: false })),
    });
    renderWithProviders(<ModelsPanel onChoose={vi.fn()} />);
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Download Qwen3.5 4B (2.74 GB)",
      }),
    );
    expect(commands.downloadModel).not.toHaveBeenCalled();
    expect(
      await screen.findByText(
        /This downloads 2\.74 GB from Hugging Face into the app's folder/,
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Download 2.74 GB" }));
    await waitFor(() =>
      expect(commands.downloadModel).toHaveBeenCalledWith("qwen3.5-4b"),
    );
  });

  it("lists the models with the recommendation and lets a downloaded one be used", async () => {
    commands.models.mockResolvedValue(view);
    const onChoose = vi.fn();
    renderWithProviders(<ModelsPanel onChoose={onChoose} />);
    expect(await screen.findByText("Recommended")).toBeInTheDocument();
    expect(
      screen.getByText("Best writing of the models tested."),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Use this one" }));
    expect(onChoose).toHaveBeenCalledWith("smollm3-3b");
  });

  it("says plainly when the computer is below the floor, and offers no download", async () => {
    commands.models.mockResolvedValue({
      ...view,
      advice: { kind: "notEnoughMemory", neededGb: 8, hasGb: 4 },
      models: view.models.map((m) => ({ ...m, installed: false })),
    });
    renderWithProviders(<ModelsPanel onChoose={vi.fn()} />);
    expect(
      await screen.findByText(
        /This computer has 4 GB of memory; the writing models need 8 GB\. Use an endpoint instead/,
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText(/suits this computer/)).not.toBeInTheDocument();
  });
});
