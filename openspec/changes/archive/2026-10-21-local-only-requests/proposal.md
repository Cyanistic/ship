# Proposal

Cyan approved this change in conversation on 2026-10-21. It is small enough to need no design papers: this proposal and the deltas under `specs/` are the contract.

## Why

The server binds only to loopback but answers any request that reaches it. A web page can make the browser send requests to it in two ways: through DNS rebinding, where the page's own hostname resolves to `127.0.0.1`, or by posting straight to `http://127.0.0.1:43179`, which the browser sends without asking even though the page can't read the reply. Either way, any page you visit can drive the server, for example with `POST /api/v0/server/stop`, which ends every program in every pane. Issue #49.

## What Changes

- Every request, `/health` included, must name a loopback host: `localhost`, an IPv4 address in `127.0.0.0/8` or `[::1]`, with any port, in the HTTP/1 `Host` header or the HTTP/2 authority. Anything else, including a missing host, gets 403.
- Any request carrying an `Origin` header gets 403. Browsers send it on the requests that can change state; the Ship client, curl and scripts don't.
- A new error kind, `forbidden`, with status 403, so the refusal has the same error body as every other failure.
- The guide says the server refuses web pages and proxies that pass on another hostname.

Not in this change:
- An access token. Without one, other user accounts on the same machine and proxies that rewrite the host to `localhost` (nginx's default `proxy_pass`) can still reach the server. Both stay open in #49.
- Support for Tailscale Serve, Caddy, cloudflared and other proxies that keep the original hostname. They're refused until the token exists.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `health-exchange`: a new requirement for refusing requests that don't come from a local, non-browser client.

## Impact

- `ship-server`: one middleware function in `middleware.rs`, layered over the whole router in `lib.rs`.
- `ship-core`: `ErrorCode::Forbidden`.
- Docs: the guide's "Using Ship on another machine" section. The README doesn't change.
- Clients: none. The Ship client, `ssh -L` tunnels, curl and scripts already send a loopback host and no `Origin`.
