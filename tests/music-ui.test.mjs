import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const source = readFileSync(new URL('../web/music.js', import.meta.url), 'utf8');
const html = readFileSync(new URL('./native-ui.fixture.html', import.meta.url), 'utf8');
const tick = () => new Promise(setImmediate);
const track = { id: 'abcdefghijk', title: '<Song>', artist: 'Artist' };
function deferred() { let resolve; const promise = new Promise(yes => { resolve = yes; }); return { promise, resolve }; }
function screen(responses = {}) {
  function element(tag = '') {
    return { tag, children: [], textContent: '', hidden: false, disabled: false, value: '',
      append(...items) { this.children.push(...items); },
      replaceChildren(...items) { this.children = items.flatMap(item => item.tag === 'fragment' ? item.children : [item]); },
    };
  }
  const elements = new Map([...html.matchAll(/\bid="([^"]+)"/g)].map(([, id]) => [id, element()]));
  elements.get('track-volume').value = '70';
  const calls = [], events = new Map(), timers = new Map(); let sequence = 0;
  const document = {
    hidden: false, documentElement: { dataset: { accountVerified: 'true' } },
    getElementById: id => elements.get(id), createElement: element,
    createDocumentFragment: () => element('fragment'),
    addEventListener: (name, handler) => events.set(name, handler),
  };
  const defaults = { stop_music: null, volume: null, snapshot: { paused: false, position_seconds: 1, duration_seconds: 30 } };
  vm.runInNewContext(source, { document, window: {
    __TAURI__: { core: { invoke: async (command, args) => {
      calls.push({ command, args });
      const table = { ...defaults, ...responses };
      if (!(command in table)) throw new Error(`Unexpected command ${command}`);
      return typeof table[command] === 'function' ? table[command](args) : table[command];
    } } },
    addEventListener: (name, handler) => events.set(name, handler),
  }, setInterval: handler => { const id = ++sequence; timers.set(id, handler); return id; }, clearInterval: id => timers.delete(id) });
  const get = id => elements.get(id);
  const search = async () => { get('music-query').value = 'query'; await get('music-search').onsubmit({ preventDefault() {} }); };
  const select = () => get('music-results').children[0].children[0].onclick();
  return { get, calls, timers, events, document, search, select };
}

test('search results are bounded, use literal metadata, and play sends only a track ID', async () => {
  const ui = screen({ search_music: Array(30).fill(track), play_music: track });
  await ui.search();
  assert.equal(ui.get('music-results').children.length, 25);
  assert.equal(ui.get('music-results').children[0].children[0].children[0].textContent, '<Song>');
  await ui.select();
  const args = ui.calls.find(call => call.command === 'play_music').args;
  assert.deepEqual(Object.keys(args), ['id']); assert.equal(args.id, track.id);
  assert.equal(ui.get('playing-title').textContent, track.title);
  assert.equal(ui.get('play-pause').disabled, false);
});

test('stopping during startup cannot revive a late playback response', async () => {
  const starting = deferred();
  const ui = screen({ search_music: [track], play_music: () => starting.promise });
  await ui.search(); const playing = ui.select();
  await ui.get('stop-music').onclick();
  starting.resolve(track); await playing;
  assert.equal(ui.get('music-player').hidden, true);
  assert.equal(ui.get('play-pause').disabled, true);
  assert.equal(ui.timers.size, 0);
  assert.equal(ui.calls.filter(call => call.command === 'volume').length, 0);
});

test('a search response after sign-out cannot repopulate the library', async () => {
  const searching = deferred();
  const ui = screen({ search_music: () => searching.promise });
  const task = ui.search();
  ui.events.get('kyuutube-account')({ detail: { connected: false } });
  searching.resolve([track]); await task;
  assert.equal(ui.get('music-results').children.length, 0);
  assert.equal(ui.get('music-browser').hidden, true);
});

test('sign-out stops native audio and clears bounded result state', async () => {
  const ui = screen({ search_music: [track], play_music: track });
  await ui.search(); await ui.select();
  ui.events.get('kyuutube-account')({ detail: { connected: false } });
  await tick();
  assert.ok(ui.calls.some(call => call.command === 'stop_music'));
  assert.equal(ui.get('music-results').children.length, 0);
  assert.equal(ui.timers.size, 0);
});

test('hidden windows stop polling and resume a single timer when visible', async () => {
  const ui = screen({ search_music: [track], play_music: track });
  await ui.search(); await ui.select(); await tick();
  assert.equal(ui.timers.size, 1);
  ui.document.hidden = true; ui.events.get('visibilitychange')();
  assert.equal(ui.timers.size, 0);
  ui.document.hidden = false; ui.events.get('visibilitychange')();
  assert.equal(ui.timers.size, 1);
});

test('required advertisements are reported rather than silently treated as playback', async () => {
  const ui = screen({ search_music: [track], play_music: () => { throw new Error('This track requires ads that the native player cannot handle yet'); } });
  await ui.search(); await ui.select();
  assert.match(ui.get('music-status').textContent, /needs ads/);
  assert.equal(ui.get('play-pause').disabled, true);
  assert.equal(ui.timers.size, 0);
});
