// react-reconciler host config — maps reconciler lifecycle calls to Rust bridge ops.
// Targets react-reconciler@0.33.x (React 19).
//
// JS handler functions are stored in the `handlers` Map keyed by ElementId.
// Rust stores only bool flags (which events are registered) and calls
// globalThis.__dispatchEvent(id, type, payload) when a GPUI event fires.

import { hostGlobal } from "./bridge-channel";
import { invoke } from "./invoke";
import type { GpuiFocusEvent, GpuiInstance, GpuiKeyboardEvent, GpuiMouseEvent } from "./primitives";

const DefaultEventPriority = 32; // React 19 lane value (was 16 in React 18)

let currentUpdatePriority = 0;

// The reconciler instance for a host element is exactly the public ref value
// (`ref.current`, see GpuiInstance): the Rust-side ElementId plus focus/blur.
// Aliased so the rest of hostConfig keeps reading as `Instance`.
type Instance = GpuiInstance;

/** Build the public instance for an ElementId, wiring the focus/blur ref methods. */
function makeInstance(id: number): Instance {
  return {
    id,
    focus: () => invoke<void>("__focus|focus", { id }),
    blur: () => invoke<void>("__focus|blur", { id }),
  };
}

type Container = { readonly root: true }; // opaque non-null root container
type Props = Record<string, unknown>;

type GpuiEvent = GpuiMouseEvent | GpuiKeyboardEvent | GpuiFocusEvent;
type EventHandler = (e: GpuiEvent) => void;
type TextEventHandler = (text: string) => void;
type RegisteredHandler = EventHandler | TextEventHandler;

/** How a dispatched event reaches its JS handler: `"event"` delivers an event
 *  object (`{ type, target, ...payload }`); `"text"` delivers the raw string
 *  value (React Native convention, used by `onChangeText` / `onSubmit`). */
type EventKind = "event" | "text";

interface EventMapEntry {
  /** Bridge wire-type string — sent to Rust and used as the handler-map key. */
  type: string;
  kind: EventKind;
}

/**
 * Single source of truth for the handler-prop vocabulary. Drives prop extraction
 * (`EVENT_PROP_TO_TYPE`), the wire types handed to Rust, and `__dispatchEvent`
 * routing (`TEXT_EVENT_TYPES`).
 *
 * The Rust mirror is the `events!` table in `crates/core/src/model.rs` (mouse /
 * keyboard / focus only — `change` / `submit` are TextInput-specific and never
 * become Rust `Events` flags). The rich per-handler signatures live in the
 * `EventProps` / `TextInputProps` interfaces in `primitives.ts`, which stay the
 * authoritative type-level declarations.
 */
const EVENT_MAP = {
  onClick: { type: "click", kind: "event" },
  onMouseDown: { type: "mousedown", kind: "event" },
  onMouseUp: { type: "mouseup", kind: "event" },
  onMouseMove: { type: "mousemove", kind: "event" },
  onMouseEnter: { type: "mouseenter", kind: "event" },
  onMouseLeave: { type: "mouseleave", kind: "event" },
  onKeyDown: { type: "keydown", kind: "event" },
  onChangeText: { type: "change", kind: "text" },
  onSubmit: { type: "submit", kind: "text" },
  onFocus: { type: "focus", kind: "event" },
  onBlur: { type: "blur", kind: "event" },
} satisfies Record<string, EventMapEntry>;

/** Event prop name → bridge wire-type string. Derived from {@link EVENT_MAP}. */
const EVENT_PROP_TO_TYPE: Record<string, string> = Object.fromEntries(
  Object.entries(EVENT_MAP).map(([prop, e]) => [prop, e.type]),
);

/** Wire types whose handlers receive the raw string value rather than an event
 *  object. Derived from {@link EVENT_MAP}'s `"text"`-kind entries. */
const TEXT_EVENT_TYPES: ReadonlySet<string> = new Set(
  Object.values(EVENT_MAP)
    .filter((e) => e.kind === "text")
    .map((e) => e.type),
);

// Key = ElementId. Populated in createInstance/commitUpdate; cleared in detachDeletedInstance.
const handlers = new Map<number, Record<string, RegisteredHandler>>();

/**
 * Called by Rust when a GPUI event fires. Routes to the registered JS handler.
 *
 * `payload` fields by event kind — mouse: `{x, y}`; keydown: `{key, shift, ctrl, alt, meta}`;
 * change/submit and TextInput focus/blur: `{value}`; View/Image focus/blur: `{}` (no fields).
 * Handlers receive `{ type, target, ...payload }`, except `onChangeText` / `onSubmit` which
 * receive the string directly (React Native convention).
 */
hostGlobal.__dispatchEvent = (id, type, payload): void => {
  const handler = handlers.get(id)?.[type];
  if (!handler) return;

  if (TEXT_EVENT_TYPES.has(type)) {
    const value = payload.value;
    (handler as TextEventHandler)(typeof value === "string" ? value : "");
  } else {
    (handler as EventHandler)({ type, target: id, ...payload } as GpuiEvent);
  }
};

/** Splits props into plain props, event type strings for Rust, and handler map for JS. */
function extractHandlers(props: Props): {
  plain: Props;
  events: string[];
  map: Record<string, RegisteredHandler>;
} {
  const plain: Props = {};
  const events: string[] = [];
  const map: Record<string, RegisteredHandler> = {};

  for (const key of Object.keys(props)) {
    const eventType = EVENT_PROP_TO_TYPE[key];
    if (eventType !== undefined && typeof props[key] === "function") {
      events.push(eventType);
      map[eventType] = props[key] as RegisteredHandler;
    } else {
      plain[key] = props[key];
    }
  }

  return { plain, events, map };
}

// Test seams — exported for characterization tests only; not part of the public API.
export { EVENT_MAP, EVENT_PROP_TO_TYPE, extractHandlers, handlers };

const bridge = hostGlobal.__bridge;

const hostConfig = {
  supportsMutation: true,
  supportsPersistence: false,
  supportsHydration: false,
  isPrimaryRenderer: true,
  noTimeout: -1,
  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,

  createInstance(
    type: string,
    props: Props,
    _rootContainer: Container,
    _hostContext: object,
    _internalInstanceHandle: unknown,
  ): Instance {
    const { plain, events, map } = extractHandlers(props);
    const id = bridge.createInstance(type, plain, events);
    if (events.length > 0) handlers.set(id, map);
    return makeInstance(id);
  },

  createTextInstance(
    text: string,
    _rootContainer: Container,
    _hostContext: object,
    _internalInstanceHandle: unknown,
  ): Instance {
    const id = bridge.createText(text);
    return makeInstance(id);
  },

  appendInitialChild(parentInstance: Instance, child: Instance): void {
    bridge.appendChild(parentInstance.id, child.id);
  },

  finalizeInitialChildren(
    _instance: Instance,
    _type: string,
    _props: Props,
    _rootContainer: Container,
    _hostContext: object,
  ): boolean {
    return false; // false → no commitMount needed
  },

  shouldSetTextContent(_type: string, _props: Props): boolean {
    return false;
  },

  // Host context is unused (single global tree).
  getRootHostContext(_rootContainer: Container): object {
    return {};
  },

  getChildHostContext(
    _parentHostContext: object,
    _type: string,
    _rootContainer: Container,
  ): object {
    return {};
  },

  getPublicInstance(instance: Instance): Instance {
    return instance;
  },

  prepareForCommit(_containerInfo: Container): null {
    return null;
  },

  resetAfterCommit(_containerInfo: Container): void {
    // Rust side flushes on its own run_jobs tick.
  },

  // Called by the reconciler when mounting a portal (`createPortal`). Portals
  // share the existing root container, so there is nothing to prepare.
  preparePortalMount(_containerInfo: Container): void {},

  appendChild(parentInstance: Instance, child: Instance): void {
    bridge.appendChild(parentInstance.id, child.id);
  },

  appendChildToContainer(_container: Container, child: Instance): void {
    bridge.appendToContainer(child.id);
  },

  insertBefore(parentInstance: Instance, child: Instance, beforeChild: Instance): void {
    bridge.insertBefore(parentInstance.id, child.id, beforeChild.id);
  },

  insertInContainerBefore(_container: Container, child: Instance, beforeChild: Instance): void {
    bridge.insertInContainer(child.id, beforeChild.id);
  },

  removeChild(parentInstance: Instance, child: Instance): void {
    bridge.removeChild(parentInstance.id, child.id);
  },

  removeChildFromContainer(_container: Container, child: Instance): void {
    bridge.removeFromContainer(child.id);
  },

  // React 19 signature: (instance, type, oldProps, newProps, internalHandle)
  commitUpdate(
    instance: Instance,
    type: string,
    _prevProps: Props,
    nextProps: Props,
    _internalInstanceHandle: unknown,
  ): void {
    const { plain, events, map } = extractHandlers(nextProps);
    // `type` is passed so Rust can re-capture props for native components.
    bridge.updateProps(instance.id, plain, events, type);
    if (events.length > 0) {
      handlers.set(instance.id, map);
    } else {
      handlers.delete(instance.id);
    }
  },

  commitTextUpdate(textInstance: Instance, _oldText: string, newText: string): void {
    bridge.updateText(textInstance.id, newText);
  },

  resetTextContent(_instance: Instance): void {},

  detachDeletedInstance(instance: Instance): void {
    bridge.detachDeleted(instance.id);
    handlers.delete(instance.id);
  },

  clearContainer(_container: Container): void {
    bridge.clearContainer();
  },

  // React 19: host config owns update priority.
  getCurrentUpdatePriority(): number {
    return currentUpdatePriority;
  },

  setCurrentUpdatePriority(newPriority: number): void {
    currentUpdatePriority = newPriority;
  },

  resolveUpdatePriority(): number {
    return currentUpdatePriority || DefaultEventPriority;
  },

  shouldAttemptEagerTransition(): boolean {
    return false;
  },

  supportsMicrotasks: true,
  scheduleMicrotask: queueMicrotask,
};

export default hostConfig;
