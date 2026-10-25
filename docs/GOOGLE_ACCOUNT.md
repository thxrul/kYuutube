# Sign in and listen

1. Extract the archive and open `kyuutube-desktop-spike` (`.exe` on Windows).
2. Choose **Open YouTube Music**.
3. Use YouTube Music's own **Sign in** button, then search and play on the site.

No Google Cloud project, client credentials, cookie export, or code pasting is needed.
In-app sessions are private and are not remembered after closing the music window.
If Google rejects embedded sign-in, choose **Open in browser** in the launcher.
Your regular browser can use its existing saved account and handles its own session.

This test mode uses the official web player, not the experimental native Rust
resolver. Ads, subscriptions, and service restrictions remain with YouTube.
Live sign-in and playback have not been verified in this cloud environment.
