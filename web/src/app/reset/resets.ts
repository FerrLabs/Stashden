import { Injectable } from "@angular/core";

export const RESET_PATH = "reset-password";

export class ResetRefused extends Error {}

@Injectable({ providedIn: "root" })
export class Resets {
  async request(email: string): Promise<void> {
    await this.post("/v1/auth/password/forgot", { email });
  }

  async redeem(token: string, password: string): Promise<void> {
    await this.post("/v1/auth/password/reset", { token, password });
  }

  private async post(path: string, body: object): Promise<void> {
    const response = await fetch(`${STASHDEN_API_URL}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (response.status === 429) {
      const seconds = Number(response.headers.get("Retry-After") ?? 60);
      throw new ResetRefused(
        `Too many requests, try again in about ${seconds} seconds.`,
      );
    }
    if (!response.ok) {
      throw new ResetRefused(await explanation(response));
    }
  }
}

async function explanation(response: Response): Promise<string> {
  try {
    const { error } = (await response.json()) as { error?: string };
    if (error !== undefined && response.status < 500) {
      return `${error.charAt(0).toUpperCase()}${error.slice(1)}.`;
    }
  } catch {
    return "The server did not answer, try again later.";
  }
  return "The server did not answer, try again later.";
}
