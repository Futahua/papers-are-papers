import { describe, expect, it } from "vitest";
import { inspectElement } from "./inspect";

describe("inspectElement", () => {
  it("captures stable source identity and user-visible meaning", () => {
    const button = document.createElement("button");
    button.dataset.papersNode = "node-123";
    button.dataset.papersSource = "src/App.tsx:42";
    button.setAttribute("aria-label", "Keep this change");
    button.textContent = "Keep";
    document.body.appendChild(button);

    const selection = inspectElement(button);

    expect(selection.nodeId).toBe("node-123");
    expect(selection.source).toBe("src/App.tsx:42");
    expect(selection.ariaLabel).toBe("Keep this change");
    expect(selection.text).toBe("Keep");
  });
});
