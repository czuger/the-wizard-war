// Reloads the page when the game changes, so every player sees the others' moves.
(function () {
  const body = document.body;
  const gameId = body.dataset.gameId;
  const version = body.dataset.version;
  if (!gameId) return;
  setInterval(async function () {
    try {
      const response = await fetch('/games/' + gameId + '/version', { cache: 'no-store' });
      if (response.ok && (await response.text()) !== version) {
        window.location.reload();
      }
    } catch (error) {
      console.warn('Version check failed', error);
    }
  }, 2000);
})();
