// @gluxe/ui — headless, unstyled components for gluxe apps.
//
// Every component ships behaviour and state only — no styles, not even a
// cursor. Style by state through render-function children, e.g.
// `<Switch>{({ checked }) => …}</Switch>`, or by reading the relevant
// compound part. See each component's JSDoc for usage.

export type { Slot } from "./internal/slot";

// ---- Button ----
export { Button } from "./button";
export type { ButtonProps } from "./button";

// ---- Toggle ----
export { Toggle } from "./toggle";
export type { ToggleProps, ToggleState } from "./toggle";

// ---- Checkbox ----
export { Checkbox, CheckboxIndicator } from "./checkbox";
export type {
  CheckboxIndicatorProps,
  CheckboxProps,
  CheckboxState,
  CheckedState,
} from "./checkbox";

// ---- Switch ----
export { Switch, SwitchThumb } from "./switch";
export type { SwitchProps, SwitchState, SwitchThumbProps } from "./switch";

// ---- RadioGroup ----
export { RadioGroup, RadioGroupIndicator, RadioGroupItem } from "./radio-group";
export type {
  RadioGroupIndicatorProps,
  RadioGroupItemProps,
  RadioGroupProps,
  RadioGroupState,
  RadioItemState,
} from "./radio-group";

// ---- Disclosure (Collapsible) ----
export { Disclosure, DisclosureContent, DisclosureTrigger } from "./disclosure";
export type {
  DisclosureContentProps,
  DisclosureProps,
  DisclosureState,
  DisclosureTriggerProps,
} from "./disclosure";

// ---- Accordion ----
export { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./accordion";
export type {
  AccordionContentProps,
  AccordionItemProps,
  AccordionItemState,
  AccordionMultipleProps,
  AccordionProps,
  AccordionSingleProps,
  AccordionTriggerProps,
} from "./accordion";

// ---- Tabs ----
export { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs";
export type {
  TabsActivationMode,
  TabsContentProps,
  TabsListProps,
  TabsOrientation,
  TabsProps,
  TabsTriggerProps,
  TabsTriggerState,
} from "./tabs";
