// The streams one tab of a Player holds open through the API, each followed as the page follows its stream.
import { expect } from "@playwright/test";
import type { ServerEvent } from "contract";

import { follow, heldOpen } from "./stream";

export class Streams {
  private controller = new AbortController();
  private reading: Promise<void>[] = [];
  private snapshots = 0;
  private keepalives = 0;

  constructor(
    private readonly url: URL,
    /** The headers of every request for a stream: the Player's network and identity cookie. */
    private readonly headers: () => Record<string, string>,
    /** Sees every answer to a request for a stream, so the Player keeps the identity cookie it sets. */
    private readonly answered: (response: Response) => void,
    private readonly onEvent: (event: ServerEvent) => void,
  ) {}

  /** How many snapshots the streams have brought so far. */
  get snapshotCount(): number {
    return this.snapshots;
  }

  /** Asks for one more stream and follows it once the server accepts it; the answer is the server's response. */
  async open(): Promise<Response> {
    const response = await fetch(this.url, { headers: this.headers(), signal: this.controller.signal });

    this.answered(response);

    if (response.ok && response.body !== null) {
      this.reading.push(this.follow(response.body));
    }

    return response;
  }

  async opened(): Promise<void> {
    await expect.poll(() => this.snapshots, "the stream should open with a snapshot").toBeGreaterThan(0);
  }

  /** Waits until the streams have brought a keepalive. */
  async keptAlive(): Promise<void> {
    await this.opened();
    await expect.poll(() => this.keepalives, "the stream should bring a keepalive").toBeGreaterThan(0);
  }

  /** Waits until the latest stream ends or `milliseconds` pass, and answers how long it stayed open. */
  async held(milliseconds: number): Promise<number> {
    await this.opened();

    return heldOpen(this.reading.at(-1) ?? Promise.resolve(), milliseconds);
  }

  /** Ends every stream, so none is open until the next `open`. */
  async close(): Promise<void> {
    this.controller.abort();
    await Promise.all(this.reading);
    this.controller = new AbortController();
    this.reading = [];
    this.snapshots = 0;
    this.keepalives = 0;
  }

  /** Follows the stream, and once the server ends or loses it, opens a new one as the page does. */
  private async follow(body: ReadableStream<Uint8Array>): Promise<void> {
    const reopen = (signal: AbortSignal) => fetch(this.url, { headers: this.headers(), signal });

    await follow(
      body,
      reopen,
      this.controller.signal,
      (event) => {
        this.snapshots += event.type === "snapshot" ? 1 : 0;
        this.onEvent(event);
      },
      () => {
        this.keepalives += 1;
      },
    );
  }
}
