import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelView } from "@/bindings/ModelView";
import { ModelsPanel } from "@/components/ModelsPanel";
import { renderWithProviders } from "@/test/render";

function model(id: string, name: string, installed: boolean): ModelView {
  return {
    id,
    name,
    sizeBytes: 2_740_937_888,
    licence: "Apache-2.0",
    minRamGb: 8,
    recommended: false,
    recommendedFor: [],
    proseNote: null,
    installed,
    partialBytes: 0,
  } as unknown as ModelView;
}

const commands = vi.hoisted(() => ({
  view: null as unknown,
  models: vi.fn(),
  cancelDownload: vi.fn(),
  deleteModel: vi.fn(),
  downloadModel: vi.fn(),
  onModelDownload: vi.fn(async () => () => {}),
}));
vi.mock("@/api/commands", () => commands);

beforeEach(() => {
  vi.clearAllMocks();
  commands.models.mockImplementation(async () => commands.view);
});

const three = [
  model("qwen4", "Qwen3.5 4B", true),
  model("qwen9", "Qwen3.5 9B", false),
  model("phi", "Phi-4-mini", false),
];

describe("ModelsPanel", () => {
  it("shows only the model in use, with a way to see the others", async () => {
    commands.view = {
      models: three,
      chosenId: "qwen4",
      advice: { kind: "recommend", id: "qwen4" },
    };
    renderWithProviders(<ModelsPanel onChoose={() => {}} />);
    expect(await screen.findByText("Qwen3.5 4B")).toBeInTheDocument();
    expect(screen.queryByText("Phi-4-mini")).toBeNull();
    fireEvent.click(
      screen.getByRole("button", { name: "Show every model (3)" }),
    );
    expect(screen.getByText("Phi-4-mini")).toBeInTheDocument();
    expect(screen.getByText("Qwen3.5 9B")).toBeInTheDocument();
  });

  it("lists every model while none is in use", async () => {
    commands.view = {
      models: three.map((m) => ({ ...m, installed: false })),
      chosenId: null,
      advice: { kind: "recommend", id: "qwen4" },
    };
    renderWithProviders(<ModelsPanel onChoose={() => {}} />);
    expect(await screen.findByText("Phi-4-mini")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Show every model/ }),
    ).toBeNull();
  });
});
