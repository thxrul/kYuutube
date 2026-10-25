# kYuutube

Native desktop YouTube Music player in development. The current source uses its
own UI, Rust Music API requests, and native libmpv audio. It does not play the
YouTube Music website or launch a browser as its player.

[Riff source review and implementation limits](docs/RIFF_REVIEW.md).

Sign-in now uses a temporary Google login window. Choose **Finish sign-in** in
the app after signing in. The backend verifies the Music account and stores the
app's own session in Windows Credential Manager, macOS Keychain, or Linux Secret
Service. No Google Cloud project, exported browser cookie file, or pasted token
is required. Cookies remain in Rust; the UI receives only success/failure and
public metadata. Google may still reject embedded sign-in; live testing is needed.

Native search/streaming currently supports direct audio leases; Riff's complete
yt-dlp extraction pipeline is not ported. Cipher-only, protected, and ad-requiring
responses are refused. Live account/stream compatibility remains unverified:
the cloud proxy blocks YouTube Music. Portable audio-runtime bundling, offline
downloads, full Metrolist parity, and the memory budget remain unfinished.

Alpha 3 is an older portal experiment and is not the current native implementation.
No new release is claimed from source compilation or mock-provider tests.

See [testing status](TEST_REPORT.md), [performance](PERFORMANCE.md), and
[native environment instructions](docs/NATIVE_SPIKE.md). YouTube Music has no
official public API; unofficial integrations can break. This project is independent
of Google/YouTube. No telemetry, ad skipping, or Premium bypass is implemented.
