Official YouTube Music web-player test preview. No Google Cloud client, code pasting, cookie export, or libmpv setup is needed for this mode.

Extract the archive and launch `kyuutube-desktop-spike` (`.exe` on Windows). Choose **Open YouTube Music**, then use the site's own sign-in button. The official site handles playback, ads, and subscriptions. This mode uses the OS webview, not the experimental Rust stream resolver.

The in-app webview uses an ephemeral session: sign in again after closing it. If Google rejects embedded sign-in, choose **Open in browser**; that opens the official Music site in your normal browser with its own saved account. No user-agent spoofing, browser cookie copying, or Google client credentials are used.

Windows needs WebView2; Linux needs WebKitGTK 4.1. macOS uses WKWebView. These are unsigned executables, not installers. The older native-audio diagnostic executables/sample are included for development; their optional libmpv dependency is unrelated to this official-player mode.

Live Google sign-in and YouTube playback remain unverified here because the cloud proxy blocks YouTube Music. Please test on your own machine and report the OS, whether login completed, and whether playback works. Never send passwords, cookies, or tokens. This prerelease is published for testing at the user's request; it is not native Rust playback or full Metrolist parity. Performance limits remain unmet on the prior virtual Linux shell.
