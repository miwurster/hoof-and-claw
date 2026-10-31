import type { Player } from "./player";

/** Every Player of a test comes from a network of their own, so the per-IP limits of one test spare the next. */
let networks = 0;

/** A fresh address in 198.18.0.0/15, the range set aside for benchmarking networks (RFC 2544). */
export function freshNetwork(): string {
  networks += 1;

  return `198.18.${String(Math.floor(networks / 256) % 256)}.${String(networks % 256)}`;
}

/** Alice's network: everyone else on it shares its per-IP limits with her. */
export class Neighbours {
  constructor(private readonly join: (name: string) => Promise<Player>) {}

  /** `count` more Players on Alice's network. */
  arrive(count: number): Promise<Player[]> {
    return Promise.all(Array.from({ length: count }, (_, index) => this.join(`Neighbour ${String(index + 1)}`)));
  }
}
