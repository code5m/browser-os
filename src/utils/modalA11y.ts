export type ModalVariant = "dialog" | "confirm" | "danger" | "alert";

export const FOCUSABLE_SELECTOR = [
  "a[href]", "button:not([disabled])", 'input:not([disabled]):not([type="hidden"])', "select:not([disabled])", "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

export function ariaRoleForVariant(variant: ModalVariant): "dialog" | "alertdialog" {
  return variant === "dialog" ? "dialog" : "alertdialog";
}

export function shouldCloseOnEscape(variant: ModalVariant): boolean {
  return variant !== "alert";
}
