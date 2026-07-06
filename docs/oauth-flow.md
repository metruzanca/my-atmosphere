# AT Protocol OAuth Flow

This document explains how OAuth authentication works in My Atmosphere, from the moment a user initiates login to the point they're redirected back and authenticated.

## Overview

My Atmosphere uses the [AT Protocol OAuth PKCE](https://atproto.com/specs/oauth) (Proof Key for Code Exchange) flow with [DPoP](https://datatracker.ietf.org/doc/html/rfc9449) (Demonstration of Proof of Possession) for secure, browser-mediated authentication against a user's Personal Data Server (PDS).

The flow involves four parties:
- **User's browser** (WASM client)
- **My Atmosphere server** (Dioxus fullstack, handles server functions and OAuth orchestration)
- **Authorization server** (the user's PDS, e.g. `bsky.social`)
- **PLC directory** (`plc.directory`) for DID resolution

## Architecture: Client vs Server Roles

All authentication logic is split across two layers:

| Layer | File | Role |
|-------|------|------|
| Client (WASM) | `src/state.rs`, `src/views/login.rs`, `src/views/callback.rs` | Renders UI, redirects browser, persists session to `localStorage` |
| Server (Rust) | `src/backend/oauth.rs`, `src/server_fns.rs` | Performs DID resolution, PAR, token exchange, DPoP key management |

Dioxus fullstack bridges these via `#[server]` functions. A function annotated with `#[server]` compiles to:
- An **HTTP POST endpoint** on the server
- A **client-side RPC call** (WASM) that serializes arguments and makes the request

## Step-by-Step Flow

### Step 1: User enters their handle

**File:** `src/views/login.rs:7-28`

The `Login` component renders a text input for the user's AT Protocol handle (e.g. `alice.bsky.social`). On submit, it calls `server_fns::init_oauth_server(handle)`, which sends a POST to the server.

### Step 2: Server initiates OAuth

**File:** `src/backend/oauth.rs:104-196` — `init_oauth()`

The server performs these steps **in order**:

1. **Resolve handle to DID** (line 107): Uses `atproto_identity::resolve::resolve_subject` with DNS + HTTP resolution. A handle like `alice.bsky.social` resolves to a DID like `did:plc:abc123...`.

2. **Discover PDS endpoint** (line 111): Fetches the DID document from `https://plc.directory/{did}` and extracts the `#atproto_pds` service endpoint — this is the URL of the user's PDS.

3. **Discover authorization server** (line 114): Uses `atproto_oauth::resources::pds_resources` to fetch the PDS's OAuth metadata and discover the authorization endpoint, token endpoint, etc.

4. **Configure client** (lines 118-128): Builds the `redirect_uri` (where the user gets sent back to — `/oauth/callback`) and `client_id`:
   - **HTTPS deployments**: `client_id` is `{base_url}/oauth/client-metadata.json`, served by `client_metadata_server` (`src/server_fns.rs:62`). The authorization server may fetch this to verify the client's JWKS and redirect URIs.
   - **Local dev (HTTP)**: Uses a `localhost` loopback-style client ID with query params encoding the redirect URI and scopes.

5. **Generate PKCE** (line 129): Creates a SHA-256 code verifier + code challenge. The challenge is sent to the authorization server; only the client that knows the verifier can exchange the code for a token.

6. **Generate random state and nonce** (lines 131-132): 16-byte hex strings. `state` prevents CSRF; `nonce` prevents replay attacks (included in DPoP proofs).

7. **Generate keys** (lines 134-141):
   - A **static signing key** (P-256): Shared across all OAuth flows for this server deployment. Deterministic if `OAUTH_KEY_SEED` env var is set (64 hex chars = 32 bytes); otherwise random on startup.
   - A **fresh DPoP key** (P-256): Generated per-request. Used to prove possession of the access token in subsequent API calls.

8. **PAR (Pushed Authorization Request)** (lines 150-158): Calls `atproto_oauth::workflow::oauth_init` which:
   - Signs a JWT containing the `client_id`, scopes, PKCE challenge, etc.
   - POSTs it to the authorization server's PAR endpoint
   - Receives a `request_uri` (a short-lived reference to the pushed request)

9. **Store state** (lines 173-185): Saves everything needed for the callback phase into the `OAUTH_STATES` in-memory HashMap, keyed by the `state` string:
   ```rust
   OAUTH_STATES.insert(state, StoredOAuthState {
       oauth_request,  // The in-flight OAuth request from the library
       auth_server,    // Authorization server metadata
       pds_url,        // PDS endpoint URL
       client_id,      // Our client identifier
       redirect_uri,   // Callback URL
       signing_key,    // Static server signing key
       dpop_key,       // Fresh DPoP key for this session
       handle,         // User's handle
   });
   ```

10. **Build authorization URL** (lines 187-193): Constructs the URL the user will visit:
    ```
    {authorization_endpoint}?client_id={client_id}&request_uri={request_uri}&state={state}
    ```

11. **Returns** `OAuthInitResponse { authorization_url }` to the client.

### Step 3: User is redirected to the authorization server

**File:** `src/views/login.rs:37-53`

When `auth_url` is populated, the client renders a link: "Continue to Authorization". The user clicks it, opening their PDS's authorization page in the browser.

On the authorization page, the user logs in (if not already) and approves the app's requested scopes.

### Step 4: Authorization server redirects back

After approval, the authorization server redirects the browser to:
```
{base_url}/oauth/callback?code=...&state=...
```

The Dioxus router (`src/main.rs:20`) matches `/oauth/callback` and renders the `OAuthCallback` component.

### Step 5: OAuthCallback processes the redirect

**File:** `src/views/callback.rs:7-107`

Inside a `use_effect` (runs on mount, WASM only):

1. **Extract query params** (line 96): Uses `web_sys::window().location().search()` and `url::form_urlencoded::parse` to extract `code` and `state` from the callback URL.

2. **Call server function** (line 72): `server_fns::complete_oauth_server(code, state).await` — sends the authorization code and state to the server.

3. **On success** (lines 25-45):
   - Updates the reactive `SessionState` with DID, handle, PDS endpoint, and access token
   - Sets `is_authenticated = true`
   - Persists the session to `localStorage` via `state::save_session()` (serialized as JSON under key `my-atmosphere-session`)
   - Navigates to `Route::Home {}` (which now renders the Dashboard since the user is authenticated)

4. **On error** (lines 47-48): Shows an error message and a "Try Again" link.

### Step 6: Server completes the token exchange

**File:** `src/backend/oauth.rs:198-246` — `complete_oauth()`

1. **Verifies state** (lines 199-204): Looks up the stored `OAuthRequest` from `OAUTH_STATES` by state string. If not found (expired or replay attempt), returns an error. **Removes it from the map** so it can't be replayed.

2. **Token exchange** (lines 214-223): Calls `atproto_oauth::workflow::oauth_complete` which:
   - Sends a POST to the token endpoint with the authorization code, PKCE verifier, and a DPoP proof
   - Validates the JWT response
   - Returns an access token

3. **Extracts user DID** (lines 225-227): Gets the `sub` claim from the token response — this is the authenticated user's DID.

4. **Stores active session** (lines 229-238): Saves into `ACTIVE_SESSIONS` HashMap keyed by DID:
   ```rust
   ACTIVE_SESSIONS.insert(did, ActiveSession {
       did, handle, pds_endpoint, access_token, dpop_key
   });
   ```

5. **Returns** `SessionData { did, handle, pds_endpoint, access_token }` to the client.

### Step 7: Authenticated API calls

**File:** `src/backend/repo.rs:16-79` — `scan_apps()`

Now that the session is stored on the server, other server functions (like repo scanning) can:
1. Look up the active session by DID from `ACTIVE_SESSIONS` (line 29)
2. Generate a DPoP proof using the stored DPoP key (line 34)
3. Make authenticated requests to the PDS with `Authorization: DPoP {access_token}` and `DPoP: {dpop_token}` headers (lines 39-45)

## Key Data Structures

### Client-side: `SessionState` (`src/state.rs:1-10`)
```rust
pub struct SessionState {
    pub is_authenticated: bool,
    pub did: String,
    pub handle: String,
    pub pds_endpoint: String,
    pub access_token: String,
}
```
Shared via Dioxus Context API (`use_context_provider` / `use_context`). Persisted to `localStorage`.

### Server-side: `StoredOAuthState` (`src/backend/oauth.rs:26-35`)
Holds all state between the init and callback phases. Stored in `OAUTH_STATES` HashMap keyed by the random state string.

### Server-side: `ActiveSession` (`src/backend/oauth.rs:37-44`)
Stored after successful token exchange. Used by subsequent API calls that need DPoP-authenticated access to the user's PDS.

### `OAuthInitResponse`, `SessionData`, `ClientMetadata` (`src/types.rs`)
Serializable types shared across the client/server boundary.

## Server State (Important Notes)

Both `OAUTH_STATES` and `ACTIVE_SESSIONS` are **in-memory HashMaps** behind `LazyLock<Mutex<_>>`. This means:

- **No persistence across restarts.** All in-flight OAuth flows and active sessions are lost on server restart.
- **No horizontal scaling.** Multiple server instances can't share state.
- **`OAUTH_KEY_SEED`** env var enables deterministic key generation, so the signing key survives restarts. Set it to 64 hex characters (32 bytes).
- For production, you would want to replace these with a database or Redis.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `OAUTH_KEY_SEED` | 64 hex chars (32 bytes). Seeds the static P-256 signing key for deterministic key generation across restarts. If unset, a random key is generated on startup. |
| `BASE_URL` | The base URL of the deployment (used for constructing `client_id` and `redirect_uri`). Set via Dioxus.toml or environment. |

## Security Properties

- **PKCE** prevents authorization code interception — even if an attacker steals the authorization code, they can't exchange it without the code verifier
- **State** prevents CSRF — the state value is verified on callback to ensure the response matches the request we initiated
- **DPoP** binds access tokens to the client's key — even if a token is stolen, it can't be used without the private DPoP key
- **Nonce** prevents replay attacks in DPoP proofs
- **PAR** keeps authorization parameters off the browser URL, reducing exposure in logs and referrer headers
