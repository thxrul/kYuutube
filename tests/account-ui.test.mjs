import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('../web/account.js', import.meta.url), 'utf8');
const html = readFileSync(new URL('./native-ui.fixture.html', import.meta.url), 'utf8');
const ready = { available: true, has_session: false };
const saved = { available: true, has_session: true };
const tick = () => new Promise(setImmediate);
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function screen(responses) {
  const elements = new Map([...html.matchAll(/<[^>]+\bid="([^"]+)"[^>]*>/g)].map(([tag, id]) => [id, {
    textContent: '', hidden: /\bhidden\b/.test(tag), disabled: /\bdisabled\b/.test(tag),
    showModal() { this.open = true; }, close() { this.open = false; },
  }]));
  const calls = [];
  const context = {
    window: { dispatchEvent() {}, __TAURI__: { core: { invoke: async (command, args) => {
      calls.push({ command, args });
      if (!(command in responses)) throw new Error(`Unexpected command ${command}`);
      return typeof responses[command] === 'function' ? responses[command]() : responses[command];
    } } } },
    CustomEvent: class { constructor(type, options) { this.type = type; this.detail = options.detail; } },
    document: { documentElement: { dataset: {} }, getElementById: (id) => {
      assert.ok(elements.has(id), `Missing production element ${id}`);
      return elements.get(id);
    } },
    localStorage: { getItem() { throw new Error('Account data must not use localStorage'); }, setItem() { throw new Error('Account data must not use localStorage'); } },
  };
  vm.runInNewContext(source, context);
  return { get: (id) => elements.get(id), calls };
}

test('an unprovisioned build cannot offer a fake working login', async () => {
  const ui = screen({ account_status: { available: false, has_session: false } });
  await tick();
  assert.equal(ui.get('google-sign-in').disabled, true);
  assert.match(ui.get('account-state').textContent, /isn’t available in this build/);
  assert.deepEqual(ui.calls.map(call => call.command), ['account_status']);
});

test('one click starts browser login without configuration arguments', async () => {
  const login = deferred();
  const ui = screen({ account_status: ready, google_sign_in: () => login.promise });
  await tick();
  assert.equal(ui.get('google-sign-in').disabled, false);
  const first = ui.get('google-sign-in').onclick();
  await ui.get('google-sign-in').onclick();
  assert.equal(ui.calls.filter(call => call.command === 'google_sign_in').length, 1);
  assert.equal(ui.calls.find(call => call.command === 'google_sign_in').args, undefined);
  assert.equal(ui.get('cancel-sign-in').hidden, false);
  assert.equal(ui.get('google-sign-in').disabled, true);
  assert.doesNotMatch(ui.get('welcome-heading').textContent, /signed in/);
  login.resolve(); await first;
  assert.match(ui.get('welcome-heading').textContent, /signed in/);
  assert.equal(ui.get('disconnect-google').hidden, false);
  assert.equal(ui.get('cancel-sign-in').hidden, true);
});

test('a saved grant is verified before showing signed-in state', async () => {
  const restore = deferred();
  const ui = screen({ account_status: saved, restore_google_session: () => restore.promise });
  await tick();
  assert.doesNotMatch(ui.get('welcome-heading').textContent, /signed in/);
  assert.equal(ui.get('google-sign-in').disabled, true);
  assert.equal(ui.get('cancel-sign-in').hidden, true);
  restore.resolve(); await tick();
  assert.match(ui.get('welcome-heading').textContent, /signed in/);
});

test('a revoked saved grant returns to sign-in without claiming a session', async () => {
  let reads = 0;
  const ui = screen({
    account_status: () => ++reads === 1 ? saved : ready,
    restore_google_session: () => { throw new Error('Authorization expired or was revoked; sign in again'); },
  });
  await tick();
  assert.doesNotMatch(ui.get('welcome-heading').textContent, /signed in/);
  assert.equal(ui.get('disconnect-google').hidden, true);
  assert.equal(ui.get('google-sign-in').disabled, false);
  assert.equal(ui.get('account-state').textContent, 'Please sign in again.');
});

test('cancellation is available only during sign-in and does not show success', async () => {
  const login = deferred();
  const ui = screen({ account_status: ready, google_sign_in: () => login.promise, cancel_google_sign_in: null });
  await tick();
  await ui.get('cancel-sign-in').onclick();
  assert.equal(ui.calls.length, 1);
  const signingIn = ui.get('google-sign-in').onclick();
  await ui.get('cancel-sign-in').onclick();
  await ui.get('cancel-sign-in').onclick();
  assert.equal(ui.calls.filter(call => call.command === 'cancel_google_sign_in').length, 1);
  login.reject(new Error('Sign-in cancelled')); await signingIn;
  assert.equal(ui.get('account-state').textContent, 'Sign-in cancelled.');
  assert.doesNotMatch(ui.get('welcome-heading').textContent, /signed in/);
});

test('offline restoration keeps sign-out available without falsely claiming success', async () => {
  const ui = screen({ account_status: saved, restore_google_session: () => { throw new Error('Could not reach Google; check your connection'); } });
  await tick();
  assert.doesNotMatch(ui.get('welcome-heading').textContent, /signed in/);
  assert.equal(ui.get('disconnect-google').hidden, false);
  assert.match(ui.get('account-state').textContent, /internet connection/);
});

test('remote revoke failure still reflects successful local sign-out', async () => {
  let signedOut = false;
  const ui = screen({
    account_status: () => signedOut ? ready : saved,
    restore_google_session: null,
    disconnect_google: () => { signedOut = true; throw new Error('Local credentials removed. Google revocation failed'); },
  });
  await tick();
  await ui.get('disconnect-google').onclick();
  assert.equal(ui.get('disconnect-google').hidden, true);
  assert.equal(ui.get('google-sign-in').disabled, false);
  assert.doesNotMatch(ui.get('welcome-heading').textContent, /signed in/);
  assert.match(ui.get('account-state').textContent, /Signed out on this device/);
});

test('retry can recover after unlocking the OS store; internal errors are not rendered', async () => {
  let locked = true;
  const ui = screen({ account_status: () => { if (locked) throw new Error('Unlock your OS credential store'); return ready; }, google_sign_in: () => { throw new Error('internal fixture-sensitive-value'); } });
  await tick();
  assert.match(ui.get('account-state').textContent, /password store/);
  locked = false;
  await ui.get('retry-account').onclick();
  assert.equal(ui.get('google-sign-in').disabled, false);
  await ui.get('google-sign-in').onclick();
  assert.doesNotMatch(ui.get('account-state').textContent, /fixture-sensitive-value/);
});
