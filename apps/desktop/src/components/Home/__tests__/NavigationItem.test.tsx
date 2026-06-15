import { fireEvent, render, screen } from "@testing-library/react";
import type { Icon, IconProps } from "@phosphor-icons/react";
import { readFileSync } from "node:fs";
import { forwardRef } from "react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, test } from "vitest";

import { Navigation, type NavigationItemConfig } from "../NavigationItem";

const DashboardIcon: Icon = forwardRef<SVGSVGElement, IconProps>(
  ({ className }, ref) => (
    <svg aria-hidden="true" className={className} ref={ref} />
  ),
);

const SettingsIcon: Icon = forwardRef<SVGSVGElement, IconProps>(
  ({ className }, ref) => (
    <svg aria-hidden="true" className={className} ref={ref} />
  ),
);

function renderNavigation(items: NavigationItemConfig[]) {
  return render(
    <MemoryRouter>
      <Navigation id="test-navigation" items={items} />
    </MemoryRouter>,
  );
}

describe("Navigation", () => {
  test("keeps active-state transitions on the navigation item itself", () => {
    const css = readFileSync("src/index.css", "utf8");
    const reducedMotionBlock =
      css.match(/@media \(prefers-reduced-motion: reduce\) \{[\s\S]*?\n\}/)?.[0] ??
      "";

    expect(css).toContain(
      "box-shadow 260ms cubic-bezier(0.22, 1, 0.36, 1)",
    );
    expect(css).toMatch(
      /\.navigation-item--active\s*\{[\s\S]*background-color: #ffffff;[\s\S]*box-shadow:/,
    );
    expect(reducedMotionBlock).not.toContain(".navigation-item");
  });

  test("renders active class from json item state", () => {
    renderNavigation([
      {
        kind: "button",
        active: true,
        icon: DashboardIcon,
        id: "dashboard",
        label: "Dashboard",
        testId: "nav-dashboard",
      },
      {
        kind: "button",
        icon: SettingsIcon,
        id: "settings",
        label: "Settings",
        testId: "nav-settings",
      },
    ]);

    const activeItem = screen.getByTestId("nav-dashboard");
    const inactiveItem = screen.getByTestId("nav-settings");

    expect(activeItem).toHaveClass("navigation-item--active");
    expect(activeItem).not.toHaveAttribute("kind");
    expect(inactiveItem).not.toHaveAttribute("kind");
    expect(inactiveItem).not.toHaveClass("navigation-item--active");
  });

  test("updates persistent background state when json active state changes", () => {
    const { rerender } = renderNavigation([
      {
        kind: "button",
        active: true,
        icon: DashboardIcon,
        id: "dashboard",
        label: "Dashboard",
        testId: "nav-dashboard",
      },
      {
        kind: "button",
        icon: SettingsIcon,
        id: "settings",
        label: "Settings",
        testId: "nav-settings",
      },
    ]);

    rerender(
      <MemoryRouter>
        <Navigation
          id="test-navigation"
          items={[
            {
              kind: "button",
              icon: DashboardIcon,
              id: "dashboard",
              label: "Dashboard",
              testId: "nav-dashboard",
            },
            {
              kind: "button",
              active: true,
              icon: SettingsIcon,
              id: "settings",
              label: "Settings",
              testId: "nav-settings",
            },
          ]}
        />
      </MemoryRouter>,
    );

    expect(screen.getByTestId("nav-dashboard")).not.toHaveClass(
      "navigation-item--active",
    );
    expect(screen.getByTestId("nav-settings")).toHaveClass(
      "navigation-item--active",
    );
  });

  test("keeps the same item mounted before it becomes active", () => {
    const { rerender } = renderNavigation([
      {
        kind: "button",
        icon: DashboardIcon,
        id: "dashboard",
        label: "Dashboard",
        testId: "nav-dashboard",
      },
      {
        kind: "button",
        icon: SettingsIcon,
        id: "settings",
        label: "Settings",
        testId: "nav-settings",
      },
    ]);

    const settingsItem = screen.getByTestId("nav-settings");

    expect(settingsItem).not.toHaveClass("navigation-item--active");

    rerender(
      <MemoryRouter>
        <Navigation
          id="test-navigation"
          items={[
            {
              kind: "button",
              icon: DashboardIcon,
              id: "dashboard",
              label: "Dashboard",
              testId: "nav-dashboard",
            },
            {
              kind: "button",
              active: true,
              icon: SettingsIcon,
              id: "settings",
              label: "Settings",
              testId: "nav-settings",
            },
          ]}
        />
      </MemoryRouter>,
    );

    const activeSettingsItem = screen.getByTestId("nav-settings");

    expect(activeSettingsItem).toBe(settingsItem);
    expect(activeSettingsItem).toHaveClass("navigation-item--active");
  });

  test("keeps link background layers mounted across route active changes", () => {
    render(
      <MemoryRouter initialEntries={["/"]}>
        <Navigation
          id="test-navigation"
          items={[
            {
              kind: "link",
              end: true,
              icon: DashboardIcon,
              id: "dashboard",
              label: "Dashboard",
              testId: "nav-dashboard",
              to: "/",
            },
            {
              kind: "link",
              icon: SettingsIcon,
              id: "settings",
              label: "Settings",
              testId: "nav-settings",
              to: "/settings",
            },
          ]}
        />
        <Routes>
          <Route element={<div />} path="/" />
          <Route element={<div />} path="/settings" />
        </Routes>
      </MemoryRouter>,
    );

    const settingsItem = screen.getByTestId("nav-settings");

    expect(screen.getByTestId("nav-dashboard")).toHaveClass(
      "navigation-item--active",
    );
    expect(settingsItem).not.toHaveClass("navigation-item--active");

    fireEvent.click(screen.getByTestId("nav-settings"));

    expect(screen.getByTestId("nav-dashboard")).not.toHaveClass(
      "navigation-item--active",
    );
    expect(screen.getByTestId("nav-settings")).toBe(settingsItem);
    expect(settingsItem).toHaveClass("navigation-item--active");
  });
});
