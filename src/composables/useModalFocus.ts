import { watch, onBeforeUnmount, type Ref } from "vue";
import { FOCUSABLE_SELECTOR, ariaRoleForVariant, shouldCloseOnEscape, type ModalVariant } from "../utils/modalA11y";

export interface ModalFocusOptions {
  variant?: ModalVariant;
  onEscape?: () => void;
}

export function useModalFocus(root: Ref<HTMLElement | null>, options: ModalFocusOptions = {}): void {
  const variant = options.variant ?? "dialog";
  let previouslyFocused: HTMLElement | null = null;
  let activeEl: HTMLElement | null = null;
  let keyHandler: ((e: KeyboardEvent) => void) | null = null;

  function focusableList(el: HTMLElement): HTMLElement[] {
    return Array.from(el.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
  }

  function focusInitial(el: HTMLElement): void {
    const primary = variant === "confirm" || variant === "danger"
      ? el.querySelector<HTMLElement>("button.primary:not([disabled]), button.danger:not([disabled]), [data-modal-primary]:not([disabled])")
      : null;
    (primary ?? focusableList(el)[0] ?? el).focus();
  }

  function activate(el: HTMLElement): void {
    previouslyFocused = (document.activeElement as HTMLElement | null) ?? previouslyFocused;
    el.setAttribute("role", ariaRoleForVariant(variant));
    el.setAttribute("aria-modal", "true");
    if (!el.hasAttribute("tabindex")) el.setAttribute("tabindex", "-1");
    keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape" && shouldCloseOnEscape(variant)) {
        e.stopPropagation();
        options.onEscape?.();
        return;
      }
      if (e.key !== "Tab") return;
      const list = focusableList(el);
      if (list.length === 0) {
        e.preventDefault();
        return;
      }
      e.preventDefault();
      const idx = list.indexOf(document.activeElement as HTMLElement);
      list[e.shiftKey ? (idx <= 0 ? list.length - 1 : idx - 1) : (idx + 1) % list.length]?.focus();
    };
    el.addEventListener("keydown", keyHandler);
    activeEl = el;
    requestAnimationFrame(() => focusInitial(el));
  }

  function deactivate(): void {
    if (activeEl && keyHandler) activeEl.removeEventListener("keydown", keyHandler);
    activeEl = null;
    keyHandler = null;
    previouslyFocused?.focus?.();
    previouslyFocused = null;
  }

  watch(
    root,
    (el) => {
      if (el) activate(el);
      else deactivate();
    },
    { flush: "post" },
  );

  onBeforeUnmount(() => deactivate());
}
