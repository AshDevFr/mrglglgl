import "@testing-library/jest-dom/vitest";
import { beforeEach, vi } from "vitest";
import { initI18n } from "@/i18n";

// Tests assert on the English catalogue.
beforeEach(async () => {
  await initI18n("en");
});

// jsdom doesn't implement these browser APIs, but Mantine (and many UI libs)
// call them on mount. Provide minimal stubs so component tests can render.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// A class, not a mocked arrow function: components construct it with `new`.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
window.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;

window.scrollTo = vi.fn() as unknown as typeof window.scrollTo;

// Mantine's auto-growing textarea re-measures when web fonts load.
Object.defineProperty(document, "fonts", {
  configurable: true,
  value: {
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    ready: Promise.resolve(),
  },
});

// Mantine's combobox scrolls the selected option into view on a timer after it opens.
Element.prototype.scrollIntoView = vi.fn();

// fireEvent doesn't wrap async UI-library state updates (Mantine popovers,
// floating-ui positioning, etc.) in act(), even though the tests still await
// the visible outcome via findBy*/waitFor. The resulting "not wrapped in
// act(...)" lines are noise rather than real bugs, so suppress just that one
// message and let everything else through.
const originalConsoleError = console.error;
console.error = (...args: unknown[]) => {
  const first = args[0];
  if (typeof first === "string" && first.includes("not wrapped in act(")) {
    return;
  }
  originalConsoleError(...args);
};
