import * as Dialog from "@radix-ui/react-dialog";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Select } from "../select";

function renderSelectInsideModal(onChange = vi.fn()) {
  render(
    <Dialog.Root open>
      <Dialog.Portal>
        <Dialog.Overlay />
        <Dialog.Content>
          <Dialog.Title>Settings</Dialog.Title>
          <Dialog.Description>Settings modal</Dialog.Description>
          <Select
            value="alpha"
            onChange={onChange}
            options={[
              { value: "alpha", label: "Alpha" },
              { value: "beta", label: "Beta" },
            ]}
          />
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>,
  );

  return onChange;
}

describe("Select", () => {
  it("keeps modal dialog options pointer-enabled", () => {
    renderSelectInsideModal();

    fireEvent.click(screen.getByRole("combobox"));

    expect(screen.getByRole("listbox")).toHaveStyle({ pointerEvents: "auto" });
  });

  it("selects an option rendered from a modal dialog", () => {
    const onChange = renderSelectInsideModal();

    fireEvent.click(screen.getByRole("combobox"));
    fireEvent.click(screen.getByRole("button", { name: /beta/i }));

    expect(onChange).toHaveBeenCalledWith({ target: { value: "beta" } });
  });
});
