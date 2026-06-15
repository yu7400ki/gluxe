// react-reconciler host config — maps reconciler lifecycle calls to Rust bridge ops.
// Targets react-reconciler@0.33.x (React 19).
//
// JS handler functions are stored in the `handlers` Map keyed by ElementId.
// Rust stores only bool flags (which events are registered) and calls
// globalThis.__dispatchEvent(id, type, payload) when a GPUI event fires.

import { invoke } from "./invoke";
import type { GpuiFocusEvent, GpuiKeyboardEvent, GpuiMouseEvent } from "./primitives";

const DefaultEventPriority = 32; // React 19 lane value (was 16 in React 18)

let currentUpdatePriority = 0;

// Lightweight wrapper carrying the Rust-side ElementId. Exposed to React as the
// `ref` value (via `getPublicInstance`), so `ref.current.focus()` works.
interface Instance {
  id: number;
  /** Move keyboard focus to this element. No-op unless the element is focusable
   *  (has `tabIndex`, `onKeyDown`, `onFocus`/`onBlur`, `autoFocus`, or `_focus*`). */
  focus(): Promise<void>;
  /** Remove keyboard focus from this element (only if it currently holds focus). */
  blur(): Promise<void>;
}

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

interface Bridge {
  createInstance(type: string, props: Props, events: string[]): number;
  createText(text: string): number;
  appendChild(parentId: number, childId: number): void;
  appendToContainer(childId: number): void;
  insertBefore(parentId: number, childId: number, beforeId: number): void;
  insertInContainer(childId: number, beforeId: number): void;
  removeChild(parentId: number, childId: number): void;
  removeFromContainer(childId: number): void;
  updateProps(id: number, props: Props, events: string[], type: string): void;
  updateText(id: number, text: string): void;
  clearContainer(): void;
  detachDeleted(id: number): void;
}

type DispatchEvent = (id: number, type: string, payload: Record<string, unknown>) => void;

interface HostGlobal {
  __bridge: Bridge;
  __dispatchEvent?: DispatchEvent;
}

const hostGlobal = globalThis as unknown as HostGlobal;

/** Maps event prop names to bridge event type strings. */
const EVENT_PROP_TO_TYPE: Record<string, string> = {
  onClick: "click",
  onMouseDown: "mousedown",
  onMouseUp: "mouseup",
  onMouseMove: "mousemove",
  onMouseEnter: "mouseenter",
  onMouseLeave: "mouseleave",
  onKeyDown: "keydown",
  onChangeText: "change",
  onSubmit: "submit",
  onFocus: "focus",
  onBlur: "blur",
};

// Key = ElementId. Populated in createInstance/commitUpdate; cleared in detachDeletedInstance.
const handlers = new Map<number, Record<string, RegisteredHandler>>();

/**
 * Called by Rust when a GPUI event fires. Routes to the registered JS handler.
 *
 * `payload` fields by event kind — mouse: `{x, y}`; keydown: `{key, shift, ctrl, alt, meta}`;
 * change/submit/focus/blur: `{value}`. Handlers receive `{ type, target, ...payload }`,
 * except `onChangeText` / `onSubmit` which receive the string directly (React Native convention).
 */
hostGlobal.__dispatchEvent = (id, type, payload): void => {
  const handler = handlers.get(id)?.[type];
  if (!handler) return;

  if (type === "change" || type === "submit") {
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
