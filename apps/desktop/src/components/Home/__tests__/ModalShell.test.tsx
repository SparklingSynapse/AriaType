import { act, render, screen } from "@testing-library/react";
import * as Dialog from "@radix-ui/react-dialog";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { ModalFrame } from "../ModalShell";

const motionState = vi.hoisted(() => ({
  surfaceAnimationComplete:
    undefined as undefined | ((definition: unknown) => void),
}));

vi.mock("framer-motion", async () => {
  const React = await import("react");

  return {
    motion: {
      div: React.forwardRef<HTMLDivElement, Record<string, unknown>>(
        ({ children, onAnimationComplete, ...props }, ref) => {
          if (props["data-testid"] === "settings-modal") {
            motionState.surfaceAnimationComplete =
              onAnimationComplete as typeof motionState.surfaceAnimationComplete;
          }

          return (
            <div ref={ref} {...props}>
              {children as React.ReactNode}
            </div>
          );
        },
      ),
    },
    useIsPresent: () => true,
  };
});

function renderModal(open: boolean) {
  return render(
    <ModalFrame
      onOpenChange={() => undefined}
      open={open}
      testId="settings-modal"
    >
      <Dialog.Title>Settings</Dialog.Title>
      <Dialog.Description>Settings modal</Dialog.Description>
      <div>Modal body</div>
    </ModalFrame>,
  );
}

describe("ModalFrame close animation lifecycle", () => {
  beforeEach(() => {
    motionState.surfaceAnimationComplete = undefined;
  });

  test("keeps closing modal mounted when an open animation completion arrives late", () => {
    const { rerender } = renderModal(true);

    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();

    rerender(
      <ModalFrame
        onOpenChange={() => undefined}
        open={false}
        testId="settings-modal"
      >
        <Dialog.Title>Settings</Dialog.Title>
        <Dialog.Description>Settings modal</Dialog.Description>
        <div>Modal body</div>
      </ModalFrame>,
    );

    act(() => {
      motionState.surfaceAnimationComplete?.({ opacity: 1, y: 0, scale: 1 });
    });

    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();
  });

  test("keeps closing modal mounted and inert while closing", () => {
    const { rerender } = renderModal(true);

    rerender(
      <ModalFrame
        onOpenChange={() => undefined}
        open={false}
        testId="settings-modal"
      >
        <Dialog.Title>Settings</Dialog.Title>
        <Dialog.Description>Settings modal</Dialog.Description>
        <div>Modal body</div>
      </ModalFrame>,
    );

    expect(screen.getByTestId("settings-modal")).toHaveStyle({
      pointerEvents: "none",
    });
  });

  test("unmounts closing modal after the closed animation completes", () => {
    const { rerender } = renderModal(true);

    rerender(
      <ModalFrame
        onOpenChange={() => undefined}
        open={false}
        testId="settings-modal"
      >
        <Dialog.Title>Settings</Dialog.Title>
        <Dialog.Description>Settings modal</Dialog.Description>
        <div>Modal body</div>
      </ModalFrame>,
    );

    act(() => {
      motionState.surfaceAnimationComplete?.({ opacity: 0, y: 10, scale: 0.98 });
    });

    expect(screen.queryByTestId("settings-modal")).not.toBeInTheDocument();
  });
});
