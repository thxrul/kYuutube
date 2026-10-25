# Riff-guided native player

Reference inspected: rootscripts/riff commit `d03b2833b07420e609276937a694d9b3254009c0`.
Riff is MIT licensed. This implementation uses it as an architectural reference;
its UI, artwork, Electron runtime, server, and extractor implementation were not copied.

Riff's `src/server/server.js` sends Music WEB_REMIX search requests, resolves
streams with yt-dlp, and exposes audio to a custom renderer. Its README explicitly
says no account/login is required. It is not a sign-in implementation to transplant.
Its Electron/WebGL visuals, third-party services, certificate-check disabling,
and ad-free extraction behavior are not adopted here.

Current kYuutube source restores a custom native UI and Rust/libmpv playback:

1. The app opens a temporary, isolated Google login window only for authentication.
2. The user chooses Finish sign-in in the native UI. Rust reads the app's own
   Music-origin cookies, verifies the Music account endpoint, and stores the
   session in the OS keystore. No cookies/passwords pass through frontend IPC.
3. Music search and player requests use the saved session and origin-bound
   SAPISID request signing. No Google Cloud registration is needed for this path.
4. Song selection resolves a bounded, expiring Google media lease in Rust and
   feeds the native decoder. No official site or browser tab is used for playback.
5. Sign-out stops audio and removes the saved session. The login webview remains
   ephemeral; long-lived credentials are stored only in OS secure storage.

Windows credential-size limits are handled with bounded 1024-byte ASCII chunks (at most 2048 UTF-16 bytes) and a
commit pointer. A rejected account verification cannot replace an existing
session. Only this app's login-window cookies are read, never another browser's
profile. Remote login pages cannot invoke application commands or inject scripts
into the native UI. Google may reject embedded sign-in; no user-agent spoof or
security-check workaround is used.

## Remaining limits

Riff's complete yt-dlp resolver has not been ported. The native resolver supports
direct audio leases and refuses cipher-only, protected, or ad-requiring responses.
It does not claim Riff's all-format/ad-free behavior. Stream-format compatibility,
ads/entitlements, actual Google login, physical playback, and portable libmpv
bundling remain unfinished or unverified. The cloud proxy still returned CONNECT
403 for Music during this review, so these cannot be certified from mocked tests.
The prior memory budget also remains unmet on virtual Linux.

The portal launch mode is removed from the default app. Alpha 3 remains a
historical portal test; it must not be described as this native implementation.
New release packaging remains blocked until native Music playback is verified.
