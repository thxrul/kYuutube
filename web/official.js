(() => {
  const invoke = window.__TAURI__.core.invoke;
  const status = document.getElementById('official-status');
  for (const [id, command] of [['open-music', 'open_music'], ['open-browser', 'open_music_browser']]) {
    const button = document.getElementById(id);
    button.onclick = async () => {
      if (button.disabled) return;
      button.disabled = true; status.textContent = 'Opening…';
      try { await invoke(command); status.textContent = ''; }
      catch (_) { status.textContent = command === 'open_music' ? 'Couldn’t open the music window. Try Open in browser.' : 'Couldn’t open your browser. Please try again.'; }
      finally { button.disabled = false; }
    };
  }
})();
