// Reference section: a styled Switch built on the headless @gluxe/ui `Switch`.
//
// The headless component supplies behaviour + state only. Everything visual —
// the track colour, the sliding thumb, the transition — comes from the
// render-function children below, which read `checked` from state.

import { View } from "@gluxe/react";
import { Switch } from "@gluxe/ui";
import React, { useState } from "react";

import { theme } from "../theme";
import { Label, Row, Section } from "../ui-kit";

function StyledSwitch({
  checked,
  onCheckedChange,
}: {
  checked: boolean;
  onCheckedChange: (next: boolean) => void;
}): React.ReactElement {
  return (
    <Switch checked={checked} onCheckedChange={onCheckedChange}>
      {({ checked }) => (
        <View
          style={{
            display: "flex",
            width: 46,
            height: 26,
            borderRadius: 999,
            padding: 3,
            flexDirection: "row",
            backgroundColor: checked ? theme.accent : theme.track,
            transition: { property: "all", duration: 160, easing: "ease-out" },
            _hover: { backgroundColor: checked ? theme.accentBright : theme.borderHigh },
          }}
        >
          <Switch.Thumb>
            {() => (
              <View
                style={{
                  width: 20,
                  height: 20,
                  borderRadius: 999,
                  backgroundColor: "#ffffff",
                  marginLeft: checked ? 20 : 0,
                  transition: { property: "all", duration: 160, easing: "ease-out" },
                  boxShadow: [{ offsetY: 1, blurRadius: 3, color: "#00000055" }],
                }}
              />
            )}
          </Switch.Thumb>
        </View>
      )}
    </Switch>
  );
}

export function SwitchSection(): React.ReactElement {
  const [wifi, setWifi] = useState(true);
  const [bluetooth, setBluetooth] = useState(false);

  return (
    <Section
      title="Switch"
      description="A boolean on/off control. The track and the sliding thumb are styled here; the thumb animates by reading checked from render-function children."
    >
      <Row>
        <StyledSwitch checked={wifi} onCheckedChange={setWifi} />
        <Label>Wi-Fi {wifi ? "on" : "off"}</Label>
      </Row>
      <Row>
        <StyledSwitch checked={bluetooth} onCheckedChange={setBluetooth} />
        <Label>Bluetooth {bluetooth ? "on" : "off"}</Label>
      </Row>
      <Row>
        <View style={{ opacity: 0.45 }}>
          <Switch checked={false} disabled>
            {({ checked }) => (
              <View
                style={{
                  display: "flex",
                  width: 46,
                  height: 26,
                  borderRadius: 999,
                  padding: 3,
                  flexDirection: "row",
                  backgroundColor: checked ? theme.accent : theme.track,
                }}
              >
                <Switch.Thumb>
                  {() => (
                    <View
                      style={{
                        width: 20,
                        height: 20,
                        borderRadius: 999,
                        backgroundColor: "#ffffff",
                        marginLeft: checked ? 20 : 0,
                      }}
                    />
                  )}
                </Switch.Thumb>
              </View>
            )}
          </Switch>
        </View>
        <Label>Disabled</Label>
      </Row>
    </Section>
  );
}
