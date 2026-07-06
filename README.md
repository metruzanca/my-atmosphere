# my-atmosphere

An AT Protocol app dashboard. Log in with your Bluesky/AT Protocol handle, authenticate via OAuth, and discover every app/namespace in your AT Protocol repository.

Built with [Dioxus 0.7](https://dioxuslabs.com) (fullstack, Rust + WASM).

This repo serves primarily as a proof-of-concept for combining Dioxus with the `atproto-*` crates and can be used as a template for AT Protocol appview apps using this tech stack.

## Features

- OAuth login against your AT Protocol PDS
- Scans your repo via `com.atproto.repo.describeRepo`
- Groups collections by namespace prefix into "discovered" app cards
- Catppuccin Mocha dark theme with Tailwind CSS

## Development

```bash
dx serve
```

## Deployment

A `Dockerfile` is included that builds and serves the app. Set the `HOST_DOMAIN` or `RAILWAY_PUBLIC_DOMAIN` environment variable to configure the OAuth redirect URL. Defaults to `http://127.0.0.1:8080`.

Set `OAUTH_KEY_SEED` to a fixed 32-byte hex-encoded string (64 chars) to make DPoP signing keys deterministic across deploys. This keeps OAuth sessions alive after restarts instead of invalidating them. Generate one with `openssl rand -hex 32`. If unset, keys are randomly generated on startup (sessions are lost on redeploy).
