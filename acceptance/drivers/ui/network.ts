// The Player's network: a TCP relay between the browser and the server that the UI driver can cut and restore.
// Chromium's offline mode leaves an established stream open, so only cutting the connection loses it.
// A relay of plain TCP cannot carry TLS, so the browser reaches an HTTPS server, the deployed service, directly.
import { connect, createServer } from "node:net";
import type { AddressInfo, Server, Socket } from "node:net";

export class Network {
  private readonly sockets = new Set<Socket>();
  private up = true;

  private constructor(
    private readonly relay: Server | undefined,
    readonly baseURL: string,
  ) {}

  /** Opens a relay on a free local port to the server at `target`, or leads straight to an HTTPS `target`. */
  static async to(target: string): Promise<Network> {
    const { protocol, hostname, port } = new URL(target);

    if (protocol === "https:") {
      return new Network(undefined, target);
    }

    let network: Network | undefined;

    const relay = createServer((browser) => {
      network?.carry(browser, hostname, Number(port));
    });

    await new Promise<void>((resolve) => {
      relay.listen(0, "127.0.0.1", resolve);
    });

    // SAFETY: a server listening on a TCP port reports an AddressInfo, never a pipe name or null.
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Node types every server's address as a union.
    const address = relay.address() as AddressInfo;

    network = new Network(relay, `http://127.0.0.1:${String(address.port)}`);

    return network;
  }

  /** Cuts every open connection, and every new one until `restore`. */
  cut(): void {
    if (this.relay === undefined) {
      throw new Error(`the network to ${this.baseURL} runs without a relay, so it cannot be cut`);
    }

    this.up = false;

    for (const socket of this.sockets) {
      socket.destroy();
    }
  }

  restore(): void {
    this.up = true;
  }

  async close(): Promise<void> {
    const { relay } = this;

    if (relay === undefined) {
      return;
    }

    this.cut();

    await new Promise<void>((resolve) => {
      relay.close(() => {
        resolve();
      });
    });
  }

  private carry(browser: Socket, hostname: string, port: number): void {
    if (!this.up) {
      browser.destroy();

      return;
    }

    const server = connect(port, hostname);

    for (const socket of [browser, server]) {
      this.sockets.add(socket);
      socket.on("close", () => {
        this.sockets.delete(socket);
        browser.destroy();
        server.destroy();
      });
      socket.on("error", () => {
        socket.destroy();
      });
    }

    browser.pipe(server);
    server.pipe(browser);
  }
}
