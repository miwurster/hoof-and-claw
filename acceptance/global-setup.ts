// Builds the SPA and the server with the test controls once, so every worker's `mise run launch` finds them built.
import { execFileSync } from "node:child_process";
import { join } from "node:path";

export default function globalSetup(): void {
  execFileSync("mise", ["run", "build:launch"], { cwd: join(import.meta.dirname, ".."), stdio: "inherit" });
}
