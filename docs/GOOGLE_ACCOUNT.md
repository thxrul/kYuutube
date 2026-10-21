# Native desktop Google account diagnostic

This is a first integration step, not complete YouTube Music login or a Metrolist desktop port. The app launches the user's system browser; it never embeds the Google login page or imports browser cookies. The requested scope is `https://www.googleapis.com/auth/youtube.readonly`.

## Set up on your computer

1. In Google Cloud Console, create/select your own project and enable **YouTube Data API v3**.
2. Configure the Google Auth Platform consent screen. For an External project in Testing, add your Google account under test users. YouTube scopes may require verification for a public production app. Testing-mode refresh grants may expire after seven days.
3. Create an OAuth client of type **Desktop app**. Do not use a web, Android, or TV/device-code client for this diagnostic. The installed-app client secret cannot be treated as a confidential server secret.
4. Open kYuutube, enter the client ID and client secret, and select **Save securely**. These values are saved only in your OS keystore. Never paste them, tokens, or cookies into chat or issue reports. The secret field is cleared after submission.
5. Select **Sign in with Google**. Finish consent in the system browser. Google redirects to a temporary `127.0.0.1` listener on a randomly assigned port. The callback is time-limited to five minutes, validates state/host/path, and uses PKCE. It closes when the operation completes or is cancelled.
6. Return to the application. A successful sign-in checks your YouTube channel. **Check my playlists** fetches a bounded page of read-only YouTube playlists. This does not grant access to the full Music library or prove Music playback.
7. **Disconnect and forget client** revokes the saved Google grant and removes local credentials. If Google is unreachable, local credentials are still removed and the app reports that remote revocation must be completed in your Google Account permissions.

Windows needs WebView2. macOS uses Keychain. Linux needs a running, unlocked Secret Service (for example GNOME Keyring or a compatible password manager). A missing or locked keystore causes an error; there is no file-storage fallback. Credentials are stored under service `com.thxrul.kyuutube.google-desktop` with separate `client` and `refresh` entries. Access tokens stay in process memory. No Google password is entered in the application.

The account screen does not require libmpv or a local audio sample. Those dependencies are needed only for the separate **Play sample** diagnostic. Neither streaming nor offline downloads are included in this iteration. Use the release's included sample instructions for audio tests.

## Report test results safely

Report the operating system, whether the browser opened, whether the callback completed, whether a channel name/playlists appeared, and the displayed error message if any. Do not include client credentials, authorization URLs, callback URLs, browser cookies, or token responses. A successful account check is still not a successful YouTube Music playback check.

## Verification limits

Local automated tests use fictitious credentials and mock token/API responses plus real loopback sockets. They test protocol and lifecycle behavior, not Google's consent service or actual Windows/macOS credential stores. The virtual Linux UI test can verify validation and missing-keystore behavior. Live consent requires the user's client and interactive browser. The cloud cannot reach the Music origin through its current proxy.

References: [Google desktop OAuth](https://developers.google.com/identity/protocols/oauth2/native-app), [YouTube API OAuth](https://developers.google.com/youtube/v3/guides/auth/installed-apps), [YouTube playlists API](https://developers.google.com/youtube/v3/docs/playlists/list).
