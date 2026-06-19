# @gluxe/ui

Headless, unstyled UI components for [gluxe](../../README.md) apps. Each
component ships **behavior and state only** — no styles, not even a cursor. You
bring the look; the library handles the interaction, the controlled/uncontrolled
state, and the wiring between parts.

Built on the `@gluxe/react` primitives (`View`, `Text`, …), so everything renders
through the native GPUI runtime.

## Install

```sh
npm install @gluxe/ui @gluxe/react react
```

## How styling works

gluxe renders to GPUI, which has no CSS attribute selectors (`[data-state]`) and
no global stylesheets. So instead of styling by attribute, you read state in JS
through **render-function children** and apply inline styles yourself:

```tsx
import { Switch } from "@gluxe/ui";
import { View } from "@gluxe/react";

<Switch defaultChecked={false}>
  {({ checked }) => (
    <View
      style={{
        width: 44,
        padding: 2,
        borderRadius: 999,
        backgroundColor: checked ? "#3d5a80" : "#ccc",
      }}
    >
      <Switch.Thumb>
        {() => (
          <View
            style={{
              width: 20,
              height: 20,
              borderRadius: 999,
              backgroundColor: "white",
              marginLeft: checked ? 22 : 0,
              transition: { property: "all", duration: 150 },
            }}
          />
        )}
      </Switch.Thumb>
    </View>
  )}
</Switch>;
```

Every part also accepts plain `<View>` props (`style`, `onClick`, …) — those are
forwarded as-is, and your `onClick` runs **before** the component's own handler.

## Keyboard & focus

Every interactive part is keyboard-operable and styles its own focus state through
the `_focus` / `_focusVisible` style props (prefer `_focusVisible` for focus rings
so they only show during keyboard use):

- **Single controls** — `Button`, `Toggle`, `Checkbox`, `Switch`, and the
  `Disclosure` / `Accordion` triggers are reachable with Tab and activated with
  **Space** or **Enter**. A `disabled` control is removed from the Tab order.
- **`RadioGroup`** — the group is a single Tab stop (the selected item, or the
  first enabled one when nothing is selected). The **arrow keys** (any direction)
  move between items and select as they go — selection follows focus, like native
  radios — wrapping at the ends and skipping disabled items. **Home** / **End**
  jump to the first / last item.
- **`Tabs`** — the tab list is a single Tab stop. The arrow keys for the
  `orientation` (Left/Right when horizontal, Up/Down when vertical) move between
  triggers, and **Home** / **End** jump to the ends. With `activationMode="automatic"`
  (the default) focus also selects the tab; `"manual"` only moves focus and defers
  selection to Space / Enter. Set `loop={false}` to stop wrapping at the ends.

```tsx
<Tabs.Trigger
  value="overview"
  style={{ _focusVisible: { borderColor: "#3d5a80", borderWidth: 2 } }}
>
  {({ selected }) => <Text>Overview</Text>}
</Tabs.Trigger>
```

The framework has no `preventDefault`, so the parts manage their own Tab order:
passing `tabIndex` to a `RadioGroup.Item` or `Tabs.Trigger` has no effect (roving
focus owns it). Other controls accept a `tabIndex` override.

## Controlled & uncontrolled

Every stateful component works both ways:

- **Uncontrolled** — omit the value prop; optionally pass `defaultValue` /
  `defaultChecked` / `defaultOpen` / `defaultPressed`.
- **Controlled** — pass the value prop and update it in the matching
  `onValueChange` / `onCheckedChange` / `onOpenChange` / `onPressedChange`.

GPUI mouse events carry no `preventDefault`, so a component's internal handler
always runs — there is no way to cancel it from your own `onClick`.

## Components

| Component    | Parts                                       | Selection model                    |
| ------------ | ------------------------------------------- | ---------------------------------- |
| `Button`     | —                                           | stateless (`onClick`)              |
| `Toggle`     | —                                           | boolean `pressed`                  |
| `Checkbox`   | `Checkbox.Indicator`                        | `true \| false \| "indeterminate"` |
| `Switch`     | `Switch.Thumb`                              | boolean `checked`                  |
| `RadioGroup` | `RadioGroup.Item`, `RadioGroup.Indicator`   | single string `value`              |
| `Disclosure` | `Disclosure.Trigger`, `Disclosure.Content`  | boolean `open`                     |
| `Accordion`  | `Accordion.Item`, `.Trigger`, `.Content`    | `single` or `multiple`             |
| `Tabs`       | `Tabs.List`, `Tabs.Trigger`, `Tabs.Content` | single string `value`              |

Indicators and collapsible content **unmount** when not active (e.g.
`Disclosure.Content` renders nothing while closed; `RadioGroup.Indicator` renders
nothing unless its item is selected). `Switch.Thumb` is the exception — it always
renders so you can animate it by position.

### Examples

```tsx
import { Button, Checkbox, RadioGroup, Disclosure, Accordion, Tabs } from "@gluxe/ui";
import { Text, View } from "@gluxe/react";

// Button — stateless pressable; onClick fires on click or Space/Enter when focused
<Button onClick={save} style={{ padding: 8 }}>
  <Text>Save</Text>
</Button>;

// Checkbox — indicator shows when checked or indeterminate
<Checkbox defaultChecked={false} onCheckedChange={(next) => console.log(next)}>
  <Checkbox.Indicator>
    {({ checked }) => <Text>{checked === "indeterminate" ? "–" : "✓"}</Text>}
  </Checkbox.Indicator>
</Checkbox>;

// RadioGroup
<RadioGroup defaultValue="b">
  <RadioGroup.Item value="a">
    <Text>Option A</Text>
    <RadioGroup.Indicator>
      <Text>●</Text>
    </RadioGroup.Indicator>
  </RadioGroup.Item>
  <RadioGroup.Item value="b">
    <Text>Option B</Text>
    <RadioGroup.Indicator>
      <Text>●</Text>
    </RadioGroup.Indicator>
  </RadioGroup.Item>
</RadioGroup>;

// Disclosure (collapsible)
<Disclosure>
  <Disclosure.Trigger>
    {({ open }) => <Text>{open ? "▾ Details" : "▸ Details"}</Text>}
  </Disclosure.Trigger>
  <Disclosure.Content>
    <Text>Hidden until opened.</Text>
  </Disclosure.Content>
</Disclosure>;

// Accordion — one open at a time, closable
<Accordion type="single" collapsible defaultValue="faq-1">
  <Accordion.Item value="faq-1">
    <Accordion.Trigger>
      <Text>Question 1</Text>
    </Accordion.Trigger>
    <Accordion.Content>
      <Text>Answer 1</Text>
    </Accordion.Content>
  </Accordion.Item>
  <Accordion.Item value="faq-2">
    <Accordion.Trigger>
      <Text>Question 2</Text>
    </Accordion.Trigger>
    <Accordion.Content>
      <Text>Answer 2</Text>
    </Accordion.Content>
  </Accordion.Item>
</Accordion>;

// Tabs
<Tabs defaultValue="overview">
  <Tabs.List>
    <Tabs.Trigger value="overview">
      {({ selected }) => <Text style={{ fontWeight: selected ? "bold" : "normal" }}>Overview</Text>}
    </Tabs.Trigger>
    <Tabs.Trigger value="settings">
      {({ selected }) => <Text style={{ fontWeight: selected ? "bold" : "normal" }}>Settings</Text>}
    </Tabs.Trigger>
  </Tabs.List>
  <Tabs.Content value="overview">
    <Text>Overview panel</Text>
  </Tabs.Content>
  <Tabs.Content value="settings">
    <Text>Settings panel</Text>
  </Tabs.Content>
</Tabs>;
```

## Notes & limitations

- **Keyboard navigation is built in.** Tab/Space/Enter activate every control,
  and `RadioGroup` / `Tabs` implement roving-focus arrow-key navigation (see
  [Keyboard & focus](#keyboard--focus) above). Items are ordered by mount order,
  which matches source order for the static lists these components target.
- **`disabled`** blocks a component's own click behavior (and its `onChange`
  callback) and removes the control from the Tab order. It does not apply any
  visual style — read `disabled` from render-function children to style it.
- **Accordion `type`** is a discriminated prop: `single` uses a string `value`
  (`collapsible` controls whether the open item can be re-clicked closed);
  `multiple` uses a `string[]` `value`.
