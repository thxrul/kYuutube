// Only public account metadata is returned over IPC. Never put credentials into
// localStorage, a URL, console output, or a rendered error message.
(() => {
  const invoke = window.__TAURI__.core.invoke;
  const $ = (id) => document.getElementById(id);
  let configured = false;
  let connected = false;
  let busy = false;
  let cursor = null;
  const status = (text) => { $('account-state').textContent = text; };
  function controls() {
    $('google-sign-in').disabled = busy || !configured;
    $('save-google').disabled = busy;
    $('disconnect-google').disabled = busy || !configured;
    $('load-playlists').disabled = busy || !connected;
    $('cancel-sign-in').hidden = !busy;
    $('client-id').disabled = busy;
    $('client-secret').disabled = busy;
  }
  async function readStatus() {
    try {
      const value = await invoke('account_status');
      configured = value.configured;
      connected = value.connected;
      status(connected ? 'Google authorization saved. Check your YouTube channel or playlists below.' : configured ? 'Client saved securely. Continue in your system browser.' : 'Set up a Google Desktop app OAuth client to connect.');
    } catch (error) { status(String(error)); }
    controls();
  }
  async function run(task) {
    if (busy) return;
    busy = true;
    controls();
    try { await task(); }
    catch (error) { status(String(error)); }
    finally { busy = false; controls(); }
  }
  $('google-config').onsubmit = (event) => {
    event.preventDefault();
    run(async () => {
      const clientId = $('client-id').value.trim();
      const clientSecret = $('client-secret').value.trim();
      // Clear the secret field before IPC settles; it is never read back.
      $('client-secret').value = '';
      await invoke('configure_google', { clientId, clientSecret });
      $('client-id').value = '';
      connected = false;
      configured = true;
      $('playlist-results').replaceChildren();
      cursor = null;
      $('load-playlists').textContent = 'Check my playlists';
      status('Client saved in your OS credential store. Sign in to continue.');
    });
  };
  $('google-sign-in').onclick = () => run(async () => {
    status('Finish Google consent in your browser, then return here. Sign-in expires after five minutes.');
    const channel = await invoke('google_sign_in');
    connected = true;
    status(`Connected to YouTube channel: ${channel.title}. YouTube Music streaming is still unverified.`);
  });
  $('cancel-sign-in').onclick = async () => {
    try { await invoke('cancel_google_sign_in'); status('Cancellation requested. Waiting for the current operation to finish.'); }
    catch (error) { status(String(error)); }
  };
  $('disconnect-google').onclick = () => run(async () => {
    try { await invoke('disconnect_google'); status('Disconnected. Local credentials removed and Google grant revoked.'); }
    finally {
      // Re-read the secure-store status even if remote revocation failed.
      try {
        const value = await invoke('account_status');
        configured = value.configured; connected = value.connected;
      } catch (_) { configured = false; connected = false; }
      $('playlist-results').replaceChildren();
      cursor = null;
    }
  });
  $('load-playlists').onclick = () => run(async () => {
    const result = await invoke('youtube_playlists', { nextPage: cursor });
    // Replace each bounded page instead of growing the DOM with the whole library.
    const fragment = document.createDocumentFragment();
    for (const item of result.items) {
      const li = document.createElement('li');
      const title = document.createElement('strong'); title.textContent = item.title;
      const owner = document.createElement('span'); owner.textContent = item.subtitle;
      li.append(title, owner); fragment.append(li);
    }
    $('playlist-results').replaceChildren(fragment);
    cursor = result.next_page;
    $('load-playlists').textContent = cursor ? 'Next page' : 'Reload playlists';
    status(result.items.length ? `Showing ${result.items.length} YouTube playlists. These are read-only in this diagnostic.` : 'No YouTube playlists were returned for this account.');
  });
  readStatus();
})();
