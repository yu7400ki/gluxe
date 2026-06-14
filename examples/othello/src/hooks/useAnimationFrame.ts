import { useEffect, useRef } from "react";

/**
 * Run `onFrame(timestamp)` on every animation frame while `active` is true.
 *
 * `onFrame` is kept in a ref so the loop always sees the latest closure
 * without re-subscribing; the loop is cancelled when `active` flips false or
 * the component unmounts.
 */
export function useAnimationFrame(active: boolean, onFrame: (ts: number) => void) {
  const cb = useRef(onFrame);
  cb.current = onFrame;

  useEffect(() => {
    if (!active) return;
    let id = requestAnimationFrame(function loop(ts) {
      cb.current(ts);
      id = requestAnimationFrame(loop);
    });
    return () => cancelAnimationFrame(id);
  }, [active]);
}
