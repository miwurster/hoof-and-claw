import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

/** jsdom has no `EventSource`; a page under test opens a stream that stays silent. */
class SilentEventSource {
  readonly addEventListener = vi.fn();
  readonly close = vi.fn();
}

vi.stubGlobal("EventSource", SilentEventSource);

afterEach(() => {
  cleanup();
});
