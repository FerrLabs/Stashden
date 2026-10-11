# TLS and a reverse proxy

`docker compose up` publishes port 3001 over plain HTTP, which is fine for trying it out and not for
anything a person signs in to. Put a reverse proxy in front of it that terminates TLS.

Without TLS three things go wrong, and the third is the one people hit first:

- The session token and every password cross the network in the clear.
- WebDAV signs in with Basic auth and an app password, which is the same secret in a header on every
  request.
- Windows refuses Basic auth over plain HTTP by default, so "Map network drive" fails on a stock
  install with a message about credentials, whatever the password.

## What the proxy has to do

The server is one process that answers the web app, `/v1` (REST) and `/dav` (WebDAV) on the same
origin, so one upstream and no path rewriting is enough. What matters is what it must not do:

- **Do not cap the body.** A single `PUT` carries the whole file, and WebDAV clients send big files
  that way. nginx's default `client_max_body_size` is 1 MiB and ingress-nginx inherits it, which is
  why a file of a few megabytes fails with 413 behind a proxy that works for small ones. The web
  app, the command line and sync send anything above 8 MiB in 8 MiB chunks, so a cap above that is
  enough for them, but a WebDAV client will not.
- **Do not buffer the request.** The server checks the destination and the room left before it reads
  a body, and stops a body that runs past the quota. A proxy that buffers the whole upload first
  defeats that and needs the disk for it.
- **Let every method through.** `PATCH` (resumable uploads), and `PROPFIND`, `PROPPATCH`, `MKCOL`,
  `COPY`, `MOVE`, `LOCK` and `UNLOCK` for WebDAV. A WAF or a "safe methods only" rule breaks WebDAV
  quietly, with a 405 the client reports as a generic failure.
- **Allow long transfers.** The default 60 second read and send timeouts cut a large download or
  upload off. An hour is a reasonable ceiling.
- **Serve it at the root of its host.** `MOVE` and `COPY` name their target in a `Destination`
  header, and the server accepts it only under `/dav`. A proxy that mounts the service under a
  prefix and strips it on the way in sends clients `Destination` URLs that no longer match.

The server does not read `X-Forwarded-For`. The attempt limiter is keyed on what is being guessed (an
address, a link), not on where the guess came from, so there is nothing to forward for it.

## nginx

```nginx
server {
    listen 443 ssl;
    http2 on;
    server_name stashden.example.com;

    ssl_certificate     /etc/letsencrypt/live/stashden.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/stashden.example.com/privkey.pem;

    client_max_body_size 0;
    proxy_request_buffering off;
    proxy_buffering off;
    proxy_http_version 1.1;
    proxy_read_timeout 1h;
    proxy_send_timeout 1h;

    location / {
        proxy_pass http://127.0.0.1:3001;
    }
}
```

## Caddy

```
stashden.example.com {
    reverse_proxy 127.0.0.1:3001
}
```

Caddy gets the certificate itself, puts no limit on the body, streams requests and responses, and
allows every method, so the defaults already do what the list above asks.

## Keeping 3001 off the network

Once the proxy is in place, nothing else should reach the API directly. With the compose file, publish
the port on the loopback address only (`127.0.0.1:3001:3001` in place of `3001:3001`), or drop the
mapping and put the proxy in the same compose project, where it reaches `api:3001` over the project
network.

## Kubernetes

With the chart, set `ingress.host` and `ingress.tls.enabled`, and give the ingress what a large upload
needs. For ingress-nginx:

```yaml
ingress:
  enabled: true
  className: nginx
  host: stashden.example.com
  tls:
    enabled: true
    secretName: stashden-tls
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "0"
    nginx.ingress.kubernetes.io/proxy-request-buffering: "off"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
```

Another ingress controller names these differently, but needs the same four things.

## Windows Explorer

Over HTTPS, "Map network drive" with the `https://` address, the account's email and an app password
works. Two defaults of the Windows WebDAV client get in the way on larger setups:

- `BasicAuthLevel` under `HKLM\SYSTEM\CurrentControlSet\Services\WebClient\Parameters` is 1 by
  default, which allows Basic auth over TLS only. That is the setting you want. Do not set it to 2 to
  make plain HTTP work; terminate TLS instead.
- `FileSizeLimitInBytes` in the same key defaults to 50,000,000, so a file above 50 MB fails to open
  or copy. Raise it (the maximum is 4294967295) and restart the WebClient service if you keep large
  files there.

## The session secret

`JWT_SECRET` signs every session token. The server starts with any non-empty value, but a short or
guessable one lets anyone who can guess it mint a token for any account. Generate it, do not type it:

```bash
openssl rand -hex 32
```

Keep it in the deployment's secret store, give every start of the server the same value, and
expect that changing it signs everyone out. The server logs a warning at startup when it is shorter
than 32 characters.
