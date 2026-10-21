# kYuutube

An independent YouTube Music desktop client in development, built with Tauri,
Rust, and native audio. It is not affiliated with Google or YouTube.

The current source offers **Sign in with Google → system browser → return**.
OAuth registration belongs to the publisher; users never enter a client ID or
secret. The main screen has a short welcome, sign-in button, and preview notice.
Developer audio tests are under **About → Audio diagnostics**.

Native Music account verification, search, stream resolution, and native player
controls are now implemented experimentally. **Live authenticated YouTube Music
playback has not passed. No new release will be published until it does.** The
cloud proxy currently blocks the Music origin, and the app's publisher OAuth
registration still needs to be supplied/verified. Passing protocol tests or
playing a local HTTP fixture does not establish service compatibility.

- [User sign-in guide](docs/GOOGLE_ACCOUNT.md)
- [One-time publisher setup](docs/PUBLISHER_SETUP.md)
- [Metrolist feature inventory](docs/METROLIST_PARITY.md)
- [Test results](TEST_REPORT.md) · [Performance](PERFORMANCE.md) · [Design audit status](MD3_AUDIT.md)

YouTube Music has no official public API; unofficial integrations can break.
The experimental adapter stops on unplayable, ad-requiring, cipher-only, or
protected responses. It does not skip ads, override entitlements, import browser
cookies, or use alternate clients/token providers. Absence of an ad field does
not prove compliant free-account behavior; live account validation is still a
release requirement. Offline downloads remain requested but unimplemented.
Last.fm and Discord are excluded. No telemetry is enabled.

Refresh credentials use Windows Credential Manager, macOS Keychain, or Linux
Secret Service. Tokens and expiring signed media URLs stay in the Rust backend.
The player currently needs a separate trusted libmpv runtime; bundling and portable
end-user packages are unfinished. The previously measured virtual Linux shell
also exceeds the original memory budget. It is not a finished download-and-listen
application yet.

## Development and testing

See [native build instructions](docs/NATIVE_SPIKE.md). The prepared cloud image
runs through `bash tools/native.sh`; it retains the managed network proxy and CA
trust. Do not disable TLS verification or bypass blocked destinations.

```sh
python3 tools/make_sample.py
bash tools/native.sh cargo test --locked -p kyuutube-youtube-account
bash tools/native.sh cargo test --locked -p kyuutube-audio-spike
node --test tests/account-ui.test.mjs tests/music-ui.test.mjs
python3 -m unittest discover -s tests -p test_release_provisioning.py
```

CI compiles Windows x64, Linux x64, macOS Intel, and macOS Apple Silicon. Pull
requests build without publisher registration. Release packaging refuses builds
without verified native Music playback, including a configured OAuth build that
has only passed mocked account tests. Native physical audio, actual account
consent, two-hour playback, failure injection, and platform/throttle checks still
need completion.

## Existing releases

[Alpha 2](https://github.com/thxrul/kYuutube/releases/tag/v0.0.1-alpha.2) is the last
published diagnostic. It has the **older** per-user OAuth setup and local-file
player. The new simplified flow and experimental streaming code are newer source;
they have not been published as a release. Existing releases are unchanged.
