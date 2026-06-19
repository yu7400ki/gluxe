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
import { Checkbox, RadioGroup, Disclosure, Accordion, Tabs } from "@gluxe/ui";
import { Text, View } from "@gluxe/react";

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

- **No focus management / keyboard navigation.** GPUI has no programmatic focus
  API, so roving-focus arrow-key navigation (e.g. between `Tabs.Trigger`s) is not
  provided. Selection happens on click. `Tabs` still accepts an `orientation`
  prop and exposes it via state for your own styling.
- **`disabled`** blocks a component's own click behavior (and its `onChange`
  callback). It does not apply any visual style — read `disabled` from
  render-function children to style it.
- **Accordion `type`** is a discriminated prop: `single` uses a string `value`
  (`collapsible` controls whether the open item can be re-clicked closed);
  `multiple` uses a `string[]` `value`.
