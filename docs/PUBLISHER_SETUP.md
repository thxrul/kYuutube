> Historical OAuth research: the current native session-login path does not require this registration. See GOOGLE_ACCOUNT.md and RIFF_REVIEW.md.

# One-time publisher setup for browser sign-in

This is maintainer work. End users never create a Cloud project, enter a client
ID/secret, export cookies, or paste a code. Their flow is **open → Sign in with
Google → browser consent → return**. Native Music playback remains an experimental,
unproven integration; OAuth by itself does not provide a Music browser session
or a playable stream.

## Register kYuutube once

1. Create/select a Google Cloud project you control. Configure Google Auth Platform
   branding and its consent screen for **kYuutube**, with your real support/contact
   details. Do not imply the app is an official Google/YouTube product.
2. Enable YouTube Data API v3 for the current account/playlist integration.
3. Create an OAuth client with application type **Desktop app**. The app uses the
   system browser, PKCE S256, random state, and a temporary `127.0.0.1` callback.
   Do not substitute an Android, web-server, or TV/device client or copy another
   project's client credentials.
4. In the repository's **Settings → Secrets and variables → Actions → Secrets**,
   set `KYUUTUBE_GOOGLE_CLIENT_ID` and `KYUUTUBE_GOOGLE_CLIENT_SECRET` to that
   registration's values. Use repository settings; do not send values in chat or
   commit them. This environment's GitHub integration cannot administer or list
   those settings (HTTP 403); the repository owner must supply them.
5. Run **Feasibility builds (not a release)** from the main branch. The workflow
   passes registration into the native build and marks artifacts with a boolean
   `publisher_oauth_configured`. No registration values are returned to the UI.
6. Test real browser consent and session restoration on a user's machine. During
   Google's Testing mode, accounts must be listed as test users and refresh grants
   may expire after seven days. Public distribution needs the appropriate Google
   publishing/verification process and quota handling. Build-time configuration
   alone is not verification or a working Music playback claim.

The registered installed-app ID and its desktop client secret are distributed in
native binaries. They cannot be confidential server credentials. GitHub secrets
keep them out of build logs; they cannot prevent extraction from an installed
application. Never embed user refresh/access tokens or a confidential server key.
The current read-only scope is `https://www.googleapis.com/auth/youtube.readonly`.
Future write scopes need appropriate consent and verification.

## Build behavior

- Both registration settings present and valid: the app offers one-button browser
  sign-in and uses the OS keystore for the user's refresh grant.
- Neither setting present: development builds open normally but show a short
  unavailable message. They never ask users to configure OAuth or reuse a previous
  alpha's user-supplied client. The publication tool rejects these new artifacts. It also rejects a configured
  build until native Music playback has actually been verified.
- Only one setting present or an invalid registration: compilation fails with a
  static error that does not include the configured value.
- Pull-request builds deliberately omit registration; they cannot publish a
  configured end-user build.

A changed publisher client cannot reuse the previous client's refresh grant.
Signing out removes the user's credentials, while the application's registration
remains available for signing in again. A failed/retried browser login preserves
an existing grant when the publisher client has not changed. Saved credentials
are verified with Google and the Music account endpoint before the interface shows a signed-in account.

For local maintainer builds, supply the same two build-time environment variables
from your secure environment, then run the normal Cargo build. There is no runtime
registration file or user-facing configuration command.

## Remaining product gates

The source now includes an experimental authenticated Music account/search/player
adapter and native network audio. It still lacks a verified live Music playback path. The cloud proxy blocks the Music origin. The previously measured Linux
webview exceeds the memory budget. Offline downloads, complete native playback,
portable runtime dependencies/installers, platform/soak/throttle validation, and
Metrolist parity remain unfinished. A cleaner sign-in screen does not resolve
these separate delivery requirements; do not advertise this preview as an official
or complete download-and-listen client.

References: [Google desktop OAuth](https://developers.google.com/identity/protocols/oauth2/native-app),
[OAuth production readiness](https://developers.google.com/identity/protocols/oauth2/production-readiness/policy-compliance),
[YouTube OAuth](https://developers.google.com/youtube/v3/guides/auth/installed-apps).
