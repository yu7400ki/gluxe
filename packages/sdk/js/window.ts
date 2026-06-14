// Window API — dispatches to the built-in "__window" plugin (the "__" namespace is
// reserved so user plugins can never shadow it). Changes apply on the next pump pass.

import { invoke } from "./invoke";

/**
 * Set the window title bar text at runtime (applied on the next pump pass).
 * For the initial title, prefer `window.title` in app.json.
 */
export function setWindowTitle(title: string): Promise<void> {
  return invoke<void>("__window|setTitle", { title });
}
