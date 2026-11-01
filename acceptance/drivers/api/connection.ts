// How the API driver reaches the server: its address, and the network a client comes from.
import type { APIRequest, Browser } from "@playwright/test";

export type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
  readonly network: string;
};
