// @gluxe/ui — headless, unstyled components for gluxe apps.
//
// Every component ships behaviour and state only — no styles, not even a
// cursor. Style by state through render-function children, e.g.
// `<Switch>{({ checked }) => …}</Switch>`, or by reading the relevant
// compound part. See each component's JSDoc for usage.

export type { Slot } from "./internal/slot";

// ---- Button ----
export { Button } from "./button/button";
export type { ButtonProps } from "./button/button";

// ---- Toggle ----
export { Toggle } from "./toggle/toggle";
export type { ToggleProps, ToggleState } from "./toggle/toggle";

// ---- Checkbox ----
export { Checkbox, CheckboxIndicator } from "./checkbox/checkbox";
export type {
  CheckboxIndicatorProps,
  CheckboxProps,
  CheckboxState,
  CheckedState,
} from "./checkbox/checkbox";

// ---- Switch ----
export { Switch, SwitchThumb } from "./switch/switch";
export type { SwitchProps, SwitchState, SwitchThumbProps } from "./switch/switch";

// ---- RadioGroup ----
export { RadioGroup, RadioGroupIndicator, RadioGroupItem } from "./radio-group/radio-group";
export type {
  RadioGroupIndicatorProps,
  RadioGroupItemProps,
  RadioGroupProps,
  RadioGroupState,
  RadioItemState,
} from "./radio-group/radio-group";

// ---- Disclosure (Collapsible) ----
export { Disclosure, DisclosureContent, DisclosureTrigger } from "./disclosure/disclosure";
export type {
  DisclosureContentProps,
  DisclosureProps,
  DisclosureState,
  DisclosureTriggerProps,
} from "./disclosure/disclosure";

// ---- Accordion ----
export { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./accordion/accordion";
export type {
  AccordionContentProps,
  AccordionItemProps,
  AccordionItemState,
  AccordionMultipleProps,
  AccordionProps,
  AccordionSingleProps,
  AccordionTriggerProps,
} from "./accordion/accordion";

// ---- Tabs ----
export { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs/tabs";
export type {
  TabsActivationMode,
  TabsContentProps,
  TabsListProps,
  TabsOrientation,
  TabsProps,
  TabsTriggerProps,
  TabsTriggerState,
} from "./tabs/tabs";
