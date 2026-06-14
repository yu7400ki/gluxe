// @vitest-environment jsdom
//
// Renders through react-dom: gluxe host elements become unknown DOM tags,
// which is enough to exercise routing — matching, context wiring, Link
// clicks (bubbled to the View onClick), and re-renders.
import { Text, View } from "gluxe";
import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Link, Outlet } from "./components";
import { useHistory, useLocation, useNavigate, useParams } from "./hooks";
import { Router } from "./router";
import type { RouteObject } from "./types";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLElement;
let root: Root;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function render(element: React.ReactElement): void {
  act(() => root.render(element));
}

/** Click the (leaf) element whose text is exactly `text`; bubbles up to the Link's View. */
function click(text: string): void {
  const leaf = [...container.querySelectorAll("*")].find(
    (el) => el.children.length === 0 && el.textContent === text,
  );
  if (!leaf) throw new Error(`No element with text "${text}" in:\n${container.innerHTML}`);
  act(() => {
    leaf.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

function page(): string {
  return container.textContent ?? "";
}

function Layout() {
  const history = useHistory();
  return (
    <View>
      <Link to="/about">
        <Text>nav-about</Text>
      </Link>
      <Link to="/users/42">
        <Text>nav-user</Text>
      </Link>
      <View onClick={() => history.back()}>
        <Text>nav-back</Text>
      </View>
      <View onClick={() => history.forward()}>
        <Text>nav-forward</Text>
      </View>
      <Outlet />
    </View>
  );
}

function Home() {
  return <Text>page-home</Text>;
}

function About() {
  return (
    <View>
      <Text>page-about</Text>
      <Link to="/users/7" replace>
        <Text>replace-user</Text>
      </Link>
    </View>
  );
}

function UserDetail() {
  const { id } = useParams<{ id: string }>();
  return <Text>{`page-user-${id}`}</Text>;
}

function NotFound() {
  const location = useLocation();
  return <Text>{`page-404:${location.pathname}`}</Text>;
}

const routes: RouteObject[] = [
  {
    component: Layout,
    children: [
      { index: true, component: Home },
      { path: "about", component: About },
      { path: "users", children: [{ path: ":id", component: UserDetail }] },
      { path: "*", component: NotFound },
    ],
  },
];

describe("Router", () => {
  it("renders the index route inside the layout", () => {
    render(<Router routes={routes} />);
    expect(page()).toContain("nav-about"); // layout chrome
    expect(page()).toContain("page-home");
  });

  it("starts at the last initialEntries entry", () => {
    render(<Router routes={routes} initialEntries={["/", "/about"]} />);
    expect(page()).toContain("page-about");
  });

  it("renders the wildcard route for unknown paths", () => {
    render(<Router routes={routes} initialEntries={["/no/such/page"]} />);
    expect(page()).toContain("page-404:/no/such/page");
  });

  it("renders nothing and warns when nothing matches and there is no wildcard", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const bare: RouteObject[] = [{ index: true, component: Home }];
      render(<Router routes={bare} initialEntries={["/missing"]} />);
      expect(page()).toBe("");
      expect(warn).toHaveBeenCalledWith(expect.stringContaining('No route matched "/missing"'));
    } finally {
      warn.mockRestore();
    }
  });
});

describe("Link navigation", () => {
  it("navigates on click", () => {
    render(<Router routes={routes} />);
    click("nav-about");
    expect(page()).toContain("page-about");
    expect(page()).not.toContain("page-home");
  });

  it("provides dynamic params to the target page", () => {
    render(<Router routes={routes} />);
    click("nav-user");
    expect(page()).toContain("page-user-42");
  });

  it("runs the user onClick before navigating", () => {
    const calls: string[] = [];
    function Probe() {
      const location = useLocation();
      return (
        <View>
          <Link to="/about" onClick={() => calls.push(`clicked at ${location.pathname}`)}>
            <Text>probe-link</Text>
          </Link>
          <Text>{`at:${location.pathname}`}</Text>
        </View>
      );
    }
    const tree: RouteObject[] = [
      {
        component: Layout,
        children: [
          { index: true, component: Probe },
          { path: "about", component: About },
        ],
      },
    ];
    render(<Router routes={tree} />);
    click("probe-link");
    expect(calls).toEqual(["clicked at /"]);
    expect(page()).toContain("page-about");
  });

  it("replace does not grow the history stack", () => {
    render(<Router routes={routes} />);
    click("nav-about"); // stack: / , /about
    click("replace-user"); // stack: / , /users/7
    expect(page()).toContain("page-user-7");
    click("nav-back");
    expect(page()).toContain("page-home"); // /about was replaced away
  });
});

describe("history controls", () => {
  it("back and forward traverse the stack", () => {
    render(<Router routes={routes} />);
    click("nav-about");
    click("nav-back");
    expect(page()).toContain("page-home");
    click("nav-forward");
    expect(page()).toContain("page-about");
  });

  it("back at the start of the stack is a no-op", () => {
    render(<Router routes={routes} />);
    click("nav-back");
    expect(page()).toContain("page-home");
  });

  it("a new navigation discards forward entries", () => {
    render(<Router routes={routes} />);
    click("nav-about");
    click("nav-back");
    click("nav-user"); // truncates /about
    click("nav-forward"); // no-op: nothing ahead
    expect(page()).toContain("page-user-42");
  });
});

describe("useNavigate", () => {
  it("supports numeric deltas", () => {
    function JumpHome() {
      const navigate = useNavigate();
      return (
        <View onClick={() => navigate(-1)}>
          <Text>jump-back</Text>
        </View>
      );
    }
    const tree: RouteObject[] = [
      {
        component: Layout,
        children: [
          { index: true, component: Home },
          { path: "about", component: About },
          { path: "jump", component: JumpHome },
        ],
      },
    ];
    render(<Router routes={tree} initialEntries={["/", "/jump"]} />);
    click("jump-back");
    expect(page()).toContain("page-home");
  });
});

describe("hooks outside <Router>", () => {
  function expectRenderToThrow(element: React.ReactElement, message: RegExp): void {
    expect(() => renderToString(element)).toThrow(message);
  }

  it("useNavigate throws", () => {
    function Bare() {
      useNavigate();
      return null;
    }
    expectRenderToThrow(<Bare />, /useNavigate\(\) must be used inside a <Router>/);
  });

  it("useLocation throws", () => {
    function Bare() {
      useLocation();
      return null;
    }
    expectRenderToThrow(<Bare />, /useLocation\(\) must be used inside a <Router>/);
  });

  it("useParams throws", () => {
    function Bare() {
      useParams();
      return null;
    }
    expectRenderToThrow(<Bare />, /useParams\(\) must be used inside a route/);
  });

  it("Outlet renders nothing", () => {
    expect(renderToString(<Outlet />)).toBe("");
  });
});
