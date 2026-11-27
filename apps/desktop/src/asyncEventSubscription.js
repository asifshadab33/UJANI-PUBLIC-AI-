// Owns the lifecycle of a Tauri event listener whose registration is asynchronous.
export function subscribeWithCleanup(registration, onError = () => {}) {
  let disposed = false;
  let unlisten;

  Promise.resolve(registration)
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch((error) => {
      if (!disposed) onError(error);
    });

  return () => {
    if (disposed) return;
    disposed = true;
    const stop = unlisten;
    unlisten = undefined;
    stop?.();
  };
}
