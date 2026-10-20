export type Methods = {
  password: boolean;
  password_reset?: boolean;
  oidc: boolean;
};

export type Returned = {
  code: string;
  state: string;
};

export type Handoff = {
  port: number;
  nonce: string;
};

export type Arrival =
  | { kind: "returned"; returned: Returned }
  | { kind: "handoff"; handoff: Handoff };

const NONCE = /^[0-9a-f]{32}$/;
const LOWEST_PORT = 1024;
const HIGHEST_PORT = 65535;

export function arrivalAt(search: string): Arrival | null {
  const query = new URLSearchParams(search);

  const code = query.get("code");
  const state = query.get("state");
  if (code !== null && state !== null && code !== "" && state !== "") {
    return { kind: "returned", returned: { code, state } };
  }

  const nonce = query.get("loopback_nonce");
  const port = Number(query.get("loopback_port"));
  if (
    nonce !== null &&
    NONCE.test(nonce) &&
    Number.isInteger(port) &&
    port >= LOWEST_PORT &&
    port <= HIGHEST_PORT
  ) {
    return { kind: "handoff", handoff: { port, nonce } };
  }

  return null;
}

export function loopbackAddress(
  handoff: Handoff,
  token: string,
  expiresIn: number,
): string {
  const query = new URLSearchParams({
    token,
    expires_in: String(expiresIn),
    nonce: handoff.nonce,
  });
  return `http://127.0.0.1:${handoff.port}/?${query.toString()}`;
}

export function parseHandoff(stored: string | null): Handoff | null {
  if (stored === null) {
    return null;
  }
  try {
    const parsed = arrivalAt(`?${stored}`);
    return parsed?.kind === "handoff" ? parsed.handoff : null;
  } catch {
    return null;
  }
}

export function serializeHandoff(handoff: Handoff): string {
  return `loopback_port=${handoff.port}&loopback_nonce=${handoff.nonce}`;
}
