// Native Music commands accept IDs, never media URLs or browser credentials.
(() => {
  const invoke = window.__TAURI__.core.invoke;
  const $ = (id) => document.getElementById(id);
  let connected = false, busy = false, active = false, paused = false;
  let timer = null, refreshing = false, epoch = 0;
  const time = (seconds) => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, '0')}`;
  function message(error) {
    const value = String(error);
    if (/ads/.test(value)) return 'This track needs ads the player cannot handle yet.';
    if (/cancelled/.test(value)) return 'Playback stopped.';
    if (/not allow playback|protected|supported native/.test(value)) return 'This track can’t play in kYuutube yet.';
    if (/libmpv|platform runtime/.test(value)) return 'The audio player isn’t available in this build.';
    if (/connection|Could not reach/.test(value)) return 'Couldn’t connect. Check your internet connection.';
    return 'Couldn’t play music. Please try again.';
  }
  const status = (value) => { $('music-status').textContent = value; };
  function controls() {
    $('search-music').disabled = busy || !connected;
    $('music-query').disabled = busy || !connected;
    $('play-pause').disabled = busy || !active;
    $('track-seek').disabled = busy || !active;
    $('track-volume').disabled = busy || !active;
  }
  async function refresh() {
    if (!active || document.hidden || refreshing) return;
    refreshing = true;
    const current = epoch;
    try {
      const value = await invoke('snapshot');
      if (current !== epoch) return;
      paused = value.paused;
      $('play-pause').textContent = paused ? 'Play' : 'Pause';
      $('track-seek').max = value.duration_seconds || 1;
      $('track-seek').value = value.position_seconds || 0;
      $('track-time').textContent = time(value.position_seconds || 0);
    } catch (_) { /* Startup/stop can briefly make the engine unavailable. */ }
    finally { refreshing = false; }
  }
  function polling() {
    if (timer !== null) clearInterval(timer);
    timer = null;
    if (active && !document.hidden) { refresh(); timer = setInterval(refresh, 1000); }
  }
  async function play(track) {
    if (!connected || busy) return;
    const current = ++epoch;
    busy = true; active = false; controls(); polling();
    $('music-player').hidden = false;
    $('playing-title').textContent = track.title;
    $('playing-artist').textContent = track.artist;
    status('Starting playback…');
    try {
      const playing = await invoke('play_music', { id: track.id });
      if (current !== epoch || !connected) return;
      $('playing-title').textContent = playing.title;
      $('playing-artist').textContent = playing.artist;
      active = true; paused = false;
      try { await invoke('volume', { value: Number($('track-volume').value) }); } catch (_) { /* Keep playback usable if the device temporarily rejects volume. */ }
      if (current !== epoch) return;
      status(''); polling();
    } catch (error) { if (current === epoch) status(message(error)); }
    finally { busy = false; controls(); }
  }
  async function stop() {
    ++epoch; active = false; polling(); controls();
    try { await invoke('stop_music'); status(''); }
    catch (error) { status(message(error)); }
    $('music-player').hidden = true;
  }
  function account(value) {
    connected = value;
    $('music-browser').hidden = !connected;
    if (!connected) {
      const wasPlaying = active || busy;
      ++epoch; active = false; polling();
      $('music-results').replaceChildren();
      $('music-query').value = '';
      $('music-player').hidden = true;
      if (wasPlaying) invoke('stop_music').catch(() => {});
    }
    controls();
  }
  $('music-search').onsubmit = async (event) => {
    event.preventDefault();
    if (!connected || busy) return;
    const query = $('music-query').value.trim();
    if (!query) return;
    const current = epoch;
    busy = true; controls(); status('Searching…');
    try {
      const tracks = await invoke('search_music', { query });
      if (current !== epoch || !connected) return;
      const fragment = document.createDocumentFragment();
      for (const track of tracks.slice(0, 25)) {
        const li = document.createElement('li');
        const button = document.createElement('button'); button.type = 'button';
        const title = document.createElement('strong'); title.textContent = track.title;
        const artist = document.createElement('span'); artist.textContent = track.artist;
        button.append(title, artist); button.onclick = () => play(track);
        li.append(button); fragment.append(li);
      }
      $('music-results').replaceChildren(fragment);
      status(tracks.length ? '' : 'No songs found. Try another search.');
    } catch (error) { if (current === epoch) status(message(error)); }
    finally { busy = false; controls(); }
  };
  $('stop-music').onclick = stop;
  $('play-pause').onclick = async () => {
    try { await invoke('pause', { paused: !paused }); await refresh(); }
    catch (error) { status(message(error)); }
  };
  $('track-seek').onchange = async () => {
    try { await invoke('seek', { seconds: Number($('track-seek').value) }); await refresh(); }
    catch (error) { status(message(error)); }
  };
  $('track-volume').onchange = async () => {
    try { await invoke('volume', { value: Number($('track-volume').value) }); }
    catch (error) { status(message(error)); }
  };
  document.addEventListener('visibilitychange', polling);
  window.addEventListener('pagehide', () => { if (timer !== null) clearInterval(timer); });
  window.addEventListener('kyuutube-account', (event) => account(event.detail.connected));
  account(document.documentElement.dataset.accountVerified === 'true');
})();
