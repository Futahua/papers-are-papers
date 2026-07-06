import type { InspectSelection } from "./types";

export function inspectElement(element: HTMLElement): InspectSelection {
  const rect = element.getBoundingClientRect();
  const style = window.getComputedStyle(element);
  const text = (element.innerText || element.textContent || "").trim().slice(0, 600);

  return {
    nodeId: element.dataset.papersNode || "unknown",
    source: element.dataset.papersSource || "unknown",
    tag: element.tagName.toLowerCase(),
    role: element.getAttribute("role") || element.tagName.toLowerCase(),
    text,
    ariaLabel: element.getAttribute("aria-label") || "",
    rect: {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    },
    appearance: {
      color: style.color,
      background: style.backgroundColor,
      font: style.fontFamily,
      fontSize: style.fontSize,
      border: style.border,
    },
  };
}
