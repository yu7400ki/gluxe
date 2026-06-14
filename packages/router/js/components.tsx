import { View, type GpuiMouseEvent, type ViewProps } from "@gluxe/react";
import React, { useContext } from "react";

import { RouteContext } from "./context";
import { useNavigate } from "./hooks";

/** Renders the matched child route. Layout components place this where page content appears. */
export function Outlet(): React.ReactElement | null {
  return useContext(RouteContext)?.outlet ?? null;
}

export interface LinkProps extends ViewProps {
  /** Pathname to navigate to, e.g. `"/users/42"`. */
  to: string;
  /** Replace the current history entry instead of pushing. */
  replace?: boolean;
}

/**
 * A `<View>` that navigates on click. The user `onClick` runs first;
 * navigation is unconditional — GPUI mouse events have no `preventDefault`.
 */
export function Link({
  to,
  replace,
  onClick,
  style,
  children,
  ...rest
}: LinkProps): React.ReactElement {
  const navigate = useNavigate();
  const handleClick = (e: GpuiMouseEvent): void => {
    onClick?.(e);
    navigate(to, { replace });
  };
  return (
    <View {...rest} style={{ cursor: "pointer", ...style }} onClick={handleClick}>
      {children}
    </View>
  );
}
