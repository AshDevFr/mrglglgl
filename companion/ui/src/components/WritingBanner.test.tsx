import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Job } from "@/bindings/Job";
import type { WritingStatus } from "@/bindings/WritingStatus";
import { WritingBanner } from "@/components/WritingBanner";
import { renderWithProviders } from "@/test/render";

function job(startDay: string, state: Job["state"]): Job {
  return {
    characterId: 1,
    kind: "day",
    startDay,
    state,
    attempts: 0,
    lastError: null,
    narrator: null,
    position: state === "pending" ? 1 : null,
  };
}

function status(over: Partial<WritingStatus>): WritingStatus {
  return { paused: false, current: null, jobs: [], blocked: null, ...over };
}

describe("WritingBanner", () => {
  it("says there is no model even when nothing is waiting yet", () => {
    renderWithProviders(
      <WritingBanner status={status({ blocked: "noModel" })} />,
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      /No writing model is downloaded yet\..*Download one in Settings/,
    );
  });

  it("counts the entries waiting and the ones that failed", () => {
    renderWithProviders(
      <WritingBanner
        status={status({
          jobs: [
            job("2026-11-01", "pending"),
            job("2026-11-02", "pending"),
            job("2026-11-03", "failed"),
          ],
        })}
      />,
    );
    const banner = screen.getByRole("status");
    expect(banner).toHaveTextContent("2 entries are waiting to be written.");
    expect(banner).toHaveTextContent("1 entry could not be written");
  });

  it("stays out of the way when there is nothing to say", () => {
    renderWithProviders(<WritingBanner status={status({})} />);
    expect(screen.queryByRole("status")).toBeNull();
  });
});
