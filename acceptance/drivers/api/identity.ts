// The identity cookie as the API driver keeps it: the `Set-Cookie` value the server issued.
import { expect } from "@playwright/test";
import type { APIResponse } from "@playwright/test";

const IDENTITY = "__Host-player";

/** The identity cookie among the `Set-Cookie` values of an answer, or the one kept so far. */
export function issued(values: readonly string[], kept: string | undefined): string | undefined {
  return values.find((value) => value.startsWith(`${IDENTITY}=`)) ?? kept;
}

/** The `Set-Cookie` values of an answer to Playwright's request API. */
export function setCookies(response: APIResponse): string[] {
  return response
    .headersArray()
    .filter((header) => header.name.toLowerCase() === "set-cookie")
    .map((header) => header.value);
}

/** The server issued an identity cookie with the attributes ADR 0001 names. */
export function expectIdentityCookie(identity: string | undefined): void {
  expect(identity, "the server should issue an identity cookie").toBeDefined();

  const attributes = (identity ?? "").split(";").map((attribute) => attribute.trim());

  expect(attributes).toEqual(expect.arrayContaining(["Path=/", "HttpOnly", "Secure", "SameSite=Lax"]));
  expect(attributes.find((attribute) => attribute.startsWith("Max-Age="))).toBe("Max-Age=34560000");
}
