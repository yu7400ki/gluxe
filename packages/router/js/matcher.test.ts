import { describe, expect, it } from "vitest";

import { matchRoutes } from "./matcher";
import type { RouteObject } from "./types";

const Dummy = () => null;

function ids(routes: RouteObject[], pathname: string): string[] | null {
  const matches = matchRoutes(routes, pathname);
  return matches ? matches.map((m) => m.route.id ?? "?") : null;
}

describe("matchRoutes", () => {
  const routes: RouteObject[] = [
    {
      id: "layout",
      component: Dummy,
      children: [
        { id: "home", index: true, component: Dummy },
        { id: "about", path: "about", component: Dummy },
        {
          id: "users",
          path: "users",
          children: [
            { id: "users-index", index: true, component: Dummy },
            { id: "users-new", path: "new", component: Dummy },
            { id: "user-detail", path: ":id", component: Dummy },
          ],
        },
        { id: "not-found", path: "*", component: Dummy },
      ],
    },
  ];

  it("matches the index route at the root", () => {
    expect(ids(routes, "/")).toEqual(["layout", "home"]);
  });

  it("matches a static route", () => {
    expect(ids(routes, "/about")).toEqual(["layout", "about"]);
  });

  it("matches a nested index route", () => {
    expect(ids(routes, "/users")).toEqual(["layout", "users", "users-index"]);
  });

  it("captures dynamic params", () => {
    const matches = matchRoutes(routes, "/users/42");
    expect(matches?.map((m) => m.route.id)).toEqual(["layout", "users", "user-detail"]);
    expect(matches?.at(-1)?.params).toEqual({ id: "42" });
  });

  it("prefers static segments over dynamic ones", () => {
    expect(ids(routes, "/users/new")).toEqual(["layout", "users", "users-new"]);
  });

  it("falls back to the wildcard for unknown paths", () => {
    const matches = matchRoutes(routes, "/no/such/page");
    expect(matches?.map((m) => m.route.id)).toEqual(["layout", "not-found"]);
    expect(matches?.at(-1)?.params).toEqual({ "*": "no/such/page" });
  });

  it("prefers any concrete route over the wildcard, regardless of order", () => {
    // not-found is declared before about in this tree
    const reordered: RouteObject[] = [
      { id: "not-found", path: "*", component: Dummy },
      { id: "about", path: "about", component: Dummy },
    ];
    expect(ids(reordered, "/about")).toEqual(["about"]);
  });

  it("decodes URI-encoded dynamic segments", () => {
    const matches = matchRoutes(routes, "/users/a%20b");
    expect(matches?.at(-1)?.params).toEqual({ id: "a b" });
  });

  it("ignores a trailing slash", () => {
    expect(ids(routes, "/about/")).toEqual(["layout", "about"]);
  });

  it("reports the matched pathname per level", () => {
    const matches = matchRoutes(routes, "/users/42");
    expect(matches?.map((m) => m.pathname)).toEqual(["/", "/users", "/users/42"]);
  });

  it("returns null when nothing matches and there is no wildcard", () => {
    const noWildcard: RouteObject[] = [{ id: "about", path: "about", component: Dummy }];
    expect(matchRoutes(noWildcard, "/missing")).toBeNull();
  });

  it("does not match a layout path itself without an index child", () => {
    const tree: RouteObject[] = [
      { id: "users", path: "users", children: [{ id: "detail", path: ":id", component: Dummy }] },
    ];
    expect(matchRoutes(tree, "/users")).toBeNull();
  });

  it("matches through component-less pass-through nodes", () => {
    const tree: RouteObject[] = [
      {
        id: "group",
        path: "settings",
        children: [{ id: "profile", path: "profile", component: Dummy }],
      },
    ];
    expect(ids(tree, "/settings/profile")).toEqual(["group", "profile"]);
  });

  it("merges ancestor params into descendant matches", () => {
    const tree: RouteObject[] = [
      {
        id: "org",
        path: "orgs/:orgId",
        children: [{ id: "repo", path: "repos/:repoId", component: Dummy }],
      },
    ];
    const matches = matchRoutes(tree, "/orgs/acme/repos/7");
    expect(matches?.at(0)?.params).toEqual({ orgId: "acme" });
    expect(matches?.at(-1)?.params).toEqual({ orgId: "acme", repoId: "7" });
  });

  it("supports multi-segment path patterns", () => {
    const tree: RouteObject[] = [{ id: "deep", path: "a/b/:c", component: Dummy }];
    const matches = matchRoutes(tree, "/a/b/3");
    expect(matches?.at(-1)?.params).toEqual({ c: "3" });
  });

  it("lets a wildcard match the empty remainder", () => {
    const tree: RouteObject[] = [{ id: "splat", path: "*", component: Dummy }];
    const matches = matchRoutes(tree, "/");
    expect(matches?.map((m) => m.route.id)).toEqual(["splat"]);
    expect(matches?.at(-1)?.params).toEqual({ "*": "" });
  });

  it("decodes wildcard segments like dynamic ones", () => {
    const tree: RouteObject[] = [{ id: "splat", path: "files/*", component: Dummy }];
    const matches = matchRoutes(tree, "/files/a%20b/c");
    expect(matches?.at(-1)?.params).toEqual({ "*": "a b/c" });
  });

  it("throws when '*' is not the last segment of a path", () => {
    const tree: RouteObject[] = [{ id: "bad", path: "files/*/meta", component: Dummy }];
    expect(() => matchRoutes(tree, "/files/x/meta")).toThrow(/only allowed as the last segment/);
  });

  it("throws when an index route has a path", () => {
    const tree: RouteObject[] = [{ id: "bad", index: true, path: "about", component: Dummy }];
    expect(() => matchRoutes(tree, "/")).toThrow(/index route cannot have/);
  });

  it("throws when an index route has children", () => {
    const tree: RouteObject[] = [
      { id: "bad", index: true, children: [{ path: "x", component: Dummy }] },
    ];
    expect(() => matchRoutes(tree, "/")).toThrow(/index route cannot have/);
  });

  it("prefers a deeper nested wildcard over the root wildcard", () => {
    const tree: RouteObject[] = [
      { id: "root-404", path: "*", component: Dummy },
      { id: "users", path: "users", children: [{ id: "users-404", path: "*", component: Dummy }] },
    ];
    expect(ids(tree, "/users/missing")).toEqual(["users", "users-404"]);
  });
});
