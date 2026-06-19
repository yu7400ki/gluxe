/**
 * Compose a consumer-supplied event handler with the component's own handler.
 *
 * The consumer's handler runs first, then the internal one (matching the order
 * used across gluxe components). GPUI events carry no `preventDefault` /
 * `stopPropagation`, so the internal handler always runs — there is no way for
 * the consumer to cancel it. Wrap your own logic accordingly.
 */
export function composeEventHandlers<E>(
  consumerHandler: ((event: E) => void) | undefined,
  internalHandler: (event: E) => void,
): (event: E) => void {
  return (event: E) => {
    consumerHandler?.(event);
    internalHandler(event);
  };
}
