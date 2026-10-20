const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);
let timer = null;
let paused = false;
let active = false;
let refreshInFlight = false;
const time = (seconds) => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, '0')}`;

async function refresh() {
  if (!active || document.hidden || refreshInFlight) return;
  refreshInFlight = true;
  try {
    const state = await invoke('snapshot');
    paused = state.paused;
    $('pause').textContent = paused ? 'Resume' : 'Pause';
    $('elapsed').textContent = time(state.position_seconds || 0);
    $('seek').max = state.duration_seconds || 1;
    $('seek').value = state.position_seconds || 0;
    $('state').textContent = `${paused ? 'Paused' : 'Local playback'} · ${state.audio_output}`;
  } catch (error) { $('state').textContent = String(error); }
  finally { refreshInFlight = false; }
}

function polling() {
  if (timer !== null) clearInterval(timer);
  timer = null;
  if (active && !document.hidden) { refresh(); timer = setInterval(refresh, 1000); }
}

async function command(name, args) {
  try { await invoke(name, args); return true; }
  catch (error) { $('state').textContent = String(error); return false; }
}
$('play').onclick = async () => {
  $('play').disabled = true;
  try {
    if (await command('play_sample')) {
      active = true;
      for (const id of ['pause', 'seek', 'volume']) $(id).disabled = false;
      await command('volume', { value: Number($('volume').value) });
      polling();
    }
  } finally { $('play').disabled = false; }
};
$('pause').onclick = async () => { if (await command('pause', { paused: !paused })) await refresh(); };
$('seek').onchange = async () => { await command('seek', { seconds: Number($('seek').value) }); await refresh(); };
$('volume').onchange = () => command('volume', { value: Number($('volume').value) });
document.addEventListener('visibilitychange', polling);
window.addEventListener('pagehide', () => { if (timer !== null) clearInterval(timer); timer = null; });
