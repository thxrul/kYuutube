// Account tokens and the publisher's OAuth registration never enter the web UI.
(() => {
  const invoke = window.__TAURI__.core.invoke;
  const $ = (id) => document.getElementById(id);
  let available = false;
  let hasSession = false;
  let verified = false;
  let busy = false;
  let signingIn = false;
  let publishedState = null;
  const status = (message) => { $('account-state').textContent = message; };
  function render() {
    $('google-sign-in').disabled = busy || !available;
    $('google-sign-in').hidden = verified;
    $('cancel-sign-in').hidden = !signingIn;
    $('cancel-sign-in').disabled = !signingIn;
    $('disconnect-google').hidden = !hasSession;
    $('disconnect-google').disabled = busy;
    $('retry-account').disabled = busy;
    $('welcome-heading').textContent = verified ? 'You’re signed in.' : 'Your music. Your space.';
    $('account-intro').textContent = verified ? 'Search for something you love.' : 'Sign in with Google to connect your account.';
    document.documentElement.dataset.accountVerified = String(verified);
    if (publishedState !== verified) {
      publishedState = verified;
      window.dispatchEvent(new CustomEvent('kyuutube-account', { detail: { connected: verified } }));
    }
  }
  function messageFor(error) {
    const message = String(error);
    if (message.includes('not available in this build')) return 'Sign-in isn’t available in this build.';
    if (/credential store|credentials|securely save/i.test(message) && !message.includes('Local credentials removed')) return 'Unlock your device’s password store, then try again.';
    if (/revoked|expired|rejected authorization|Sign in with Google first/.test(message)) return 'Please sign in again.';
    if (message.includes('declined')) return 'Sign-in wasn’t completed.';
    if (message.includes('cancelled')) return 'Sign-in cancelled.';
    if (message.includes('timed out')) return 'Sign-in timed out. Try again.';
    if (message.includes('system browser')) return 'Couldn’t open your browser. Please try again.';
    if (/Could not reach|connection/.test(message)) return 'Couldn’t connect. Check your internet connection.';
    if (message.includes('Local credentials removed')) return 'Signed out on this device. Remove app access in your Google Account if needed.';
    return 'Couldn’t connect your account. Please try again.';
  }
  async function inspectSession() {
    const state = await invoke('account_status');
    available = state.available;
    hasSession = state.has_session;
  }
  async function run(task, signIn = false) {
    if (busy) return;
    busy = true;
    signingIn = signIn;
    $('retry-account').hidden = true;
    render();
    try { await task(); }
    catch (error) {
      // Reconcile persisted state after cancellation, expiry, and remote sign-out
      // failures. A saved grant alone never sets the verified UI state.
      try { await inspectSession(); } catch (_) { /* Keep the last known state. */ }
      status(messageFor(error));
      $('retry-account').hidden = false;
    } finally {
      busy = false;
      signingIn = false;
      render();
    }
  }
  const restore = () => run(async () => {
    verified = false;
    render();
    status('Connecting…');
    await inspectSession();
    if (!available) { status('Sign-in isn’t available in this build.'); return; }
    if (hasSession) {
      await invoke('restore_google_session');
      verified = true;
    }
    status('');
  });
  $('google-sign-in').onclick = () => run(async () => {
    status('Finish signing in in your browser.');
    await invoke('google_sign_in');
    hasSession = true;
    verified = true;
    status('');
  }, true);
  $('cancel-sign-in').onclick = async () => {
    if (!signingIn || $('cancel-sign-in').disabled) return;
    $('cancel-sign-in').disabled = true;
    try { await invoke('cancel_google_sign_in'); status('Cancelling…'); }
    catch (error) { status(messageFor(error)); $('cancel-sign-in').disabled = false; }
  };
  $('disconnect-google').onclick = () => run(async () => {
    verified = false;
    render();
    status('Signing out…');
    await invoke('disconnect_google');
    hasSession = false;
    status('');
  });
  $('retry-account').onclick = restore;
  $('about-open').onclick = () => $('about-dialog').showModal();
  $('about-close').onclick = () => $('about-dialog').close();
  restore();
})();
