type MessageType = "success" | "warning" | "info" | "error";

function show(type: MessageType, message: string) {
  const root = ensureRoot();
  const item = document.createElement("div");
  item.className = `cpms-toast cpms-toast--${type}`;
  item.textContent = message;
  root.appendChild(item);
  window.setTimeout(() => item.remove(), 3200);
}

function ensureRoot() {
  const existing = document.querySelector<HTMLElement>("[data-cpms-toast-root]");
  if (existing) {
    return existing;
  }

  const root = document.createElement("div");
  root.dataset.cpmsToastRoot = "true";
  root.className = "cpms-toast-root";
  document.body.appendChild(root);
  return root;
}

export const message = {
  success: (text: string) => show("success", text),
  warning: (text: string) => show("warning", text),
  info: (text: string) => show("info", text),
  error: (text: string) => show("error", text),
};

export function confirmAction(text: string) {
  return Promise.resolve(window.confirm(text));
}
