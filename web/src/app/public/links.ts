import { Injectable } from '@angular/core';
import { encodePath } from '../platform';
import { saveStreaming } from '../transfer';
import type { Linked } from '../share';
import { entriesAt, getPublic } from './public-request';

@Injectable({ providedIn: 'root' })
export class PublicLinks {
  async open(token: string, password: string, path: string): Promise<Linked> {
    const response = await this.get(token, entriesAt(path), password);
    return (await response.json()) as Linked;
  }

  async download(token: string, password: string, path: string, name: string): Promise<void> {
    await saveStreaming(() => this.get(token, `/content${encodePath(path)}`, password), name);
  }

  async downloadArchive(
    token: string,
    password: string,
    path: string,
    name: string,
  ): Promise<void> {
    await saveStreaming(
      () => this.get(token, `/archive${encodePath(path)}`, password),
      `${name}.zip`,
    );
  }

  private get(token: string, at: string, password: string): Promise<Response> {
    return getPublic(STASHDEN_API_URL, token, at, password);
  }
}
