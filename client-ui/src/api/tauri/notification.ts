import { isTauri } from "@tauri-apps/api/core";
import { emitTo, listen, once, type UnlistenFn } from "@tauri-apps/api/event";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { currentMonitor, primaryMonitor } from "@tauri-apps/api/window";
import type { AppNotification } from "@/types/app/notification";

export const DESKTOP_NOTIFICATION_WINDOW = "notification";
export const DESKTOP_NOTIFICATION_PUSH_EVENT = "desktop-notification:push";
export const DESKTOP_NOTIFICATION_READY_EVENT = "desktop-notification:ready";
export const DESKTOP_NOTIFICATION_ACK_EVENT = "desktop-notification:ack";

export type DesktopNotificationAckStage = "received" | "shown" | "show-error";

export interface DesktopNotificationAckPayload {
  id: string;
  stage: DesktopNotificationAckStage;
  error?: string;
  time: number;
}

export type DesktopNotificationDiagnosticStatus = "ok" | "failed" | "skipped" | "timeout";

export interface DesktopNotificationDiagnosticStep {
  name: string;
  status: DesktopNotificationDiagnosticStatus;
  durationMs: number;
  detail?: string;
}

export interface DesktopNotificationDiagnostic {
  ok: boolean;
  steps: DesktopNotificationDiagnosticStep[];
  ack?: DesktopNotificationAckPayload;
}

interface NotificationAckWaitResult {
  received?: DesktopNotificationAckPayload;
  final?: DesktopNotificationAckPayload;
}

interface NotificationAckWaiter {
  promise: Promise<NotificationAckWaitResult>;
  cancel: () => void;
}

const DEFAULT_ACK_TIMEOUT_MS = 2500;

// 设计约定：通知窗口为 400×400，固定在屏幕右下角。
const WINDOW_WIDTH = 400;
const WINDOW_HEIGHT = 400;
const WINDOW_MARGIN = 20;

let notificationWindowPromise: Promise<WebviewWindow> | undefined;

/** 启动时预创建（隐藏）通知子窗口，使首条通知能立即显示（DESIGN：默认加载但不显示）。 */
export async function prepareNotificationWindow() {
  if (!isTauri()) {
    return;
  }

  try {
    await ensureNotificationWindow();
  } catch (error) {
    console.warn("Failed to pre-create desktop notification window", error);
  }
}

/** 显示桌面级通知子窗口，并把通知 payload 推送给 notification 窗口。 */
export async function showDesktopNotification(notification: AppNotification) {
  if (!isTauri()) {
    return;
  }

  const notificationWindow = await ensureNotificationWindow();
  await positionNotificationWindow(notificationWindow);
  await emitTo(DESKTOP_NOTIFICATION_WINDOW, DESKTOP_NOTIFICATION_PUSH_EVENT, notification);
}

export async function diagnoseDesktopNotification(
  notification: AppNotification,
  options: { ackTimeoutMs?: number } = {},
): Promise<DesktopNotificationDiagnostic> {
  const steps: DesktopNotificationDiagnosticStep[] = [];
  const ackTimeoutMs = options.ackTimeoutMs ?? DEFAULT_ACK_TIMEOUT_MS;

  const tauriStartedAt = now();
  if (!isTauri()) {
    pushStep(steps, "tauri-runtime", "skipped", tauriStartedAt, "not running inside Tauri");
    return { ok: false, steps };
  }
  pushStep(steps, "tauri-runtime", "ok", tauriStartedAt);

  let notificationWindow: WebviewWindow;
  const windowStartedAt = now();
  try {
    notificationWindow = await ensureNotificationWindow();
    pushStep(steps, "ensure-window", "ok", windowStartedAt);
  } catch (error) {
    pushStep(steps, "ensure-window", "failed", windowStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  const positionStartedAt = now();
  try {
    await positionNotificationWindow(notificationWindow);
    pushStep(steps, "position-window", "ok", positionStartedAt);
  } catch (error) {
    pushStep(steps, "position-window", "failed", positionStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  let ackWaiter: NotificationAckWaiter;
  const ackListenerStartedAt = now();
  try {
    ackWaiter = await createNotificationAckWaiter(notification.id, ackTimeoutMs);
    pushStep(steps, "ack-listener", "ok", ackListenerStartedAt);
  } catch (error) {
    pushStep(steps, "ack-listener", "failed", ackListenerStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  const emitStartedAt = now();
  try {
    await emitTo(DESKTOP_NOTIFICATION_WINDOW, DESKTOP_NOTIFICATION_PUSH_EVENT, notification);
    pushStep(steps, "emit-push-event", "ok", emitStartedAt);
  } catch (error) {
    ackWaiter.cancel();
    pushStep(steps, "emit-push-event", "failed", emitStartedAt, errorMessage(error));
    return { ok: false, steps };
  }

  const ackStartedAt = now();
  const ackResult = await ackWaiter.promise;
  if (!ackResult.received) {
    pushStep(
      steps,
      "notification-window-received",
      "timeout",
      ackStartedAt,
      "notification window did not acknowledge the push event",
    );
    pushStep(steps, "notification-window-show", "skipped", ackStartedAt);
    return { ok: false, steps };
  }

  pushStep(steps, "notification-window-received", "ok", ackStartedAt);

  if (ackResult.final?.stage === "shown") {
    pushStep(steps, "notification-window-show", "ok", ackStartedAt);
    return { ok: true, steps, ack: ackResult.final };
  }

  if (ackResult.final?.stage === "show-error") {
    pushStep(
      steps,
      "notification-window-show",
      "failed",
      ackStartedAt,
      ackResult.final.error ?? "window.show() failed",
    );
    return { ok: false, steps, ack: ackResult.final };
  }

  pushStep(
    steps,
    "notification-window-show",
    "timeout",
    ackStartedAt,
    "notification window received the event but did not confirm show()",
  );
  return { ok: false, steps, ack: ackResult.received };
}

async function ensureNotificationWindow() {
  if (notificationWindowPromise) {
    return notificationWindowPromise;
  }

  const existingWindow = await WebviewWindow.getByLabel(DESKTOP_NOTIFICATION_WINDOW);

  if (existingWindow) {
    return existingWindow;
  }

  notificationWindowPromise ??= createNotificationWindow().catch(async (error) => {
    if (isDuplicateWindowLabelError(error)) {
      const existingWindow = await WebviewWindow.getByLabel(DESKTOP_NOTIFICATION_WINDOW);

      if (existingWindow) {
        return existingWindow;
      }
    }

    notificationWindowPromise = undefined;
    throw error;
  });
  return notificationWindowPromise;
}

async function createNotificationWindow() {
  const readyPromise = waitForEvent(DESKTOP_NOTIFICATION_READY_EVENT, 2500);
  const notificationWindow = new WebviewWindow(DESKTOP_NOTIFICATION_WINDOW, {
    url: "/",
    parent: "main",
    title: "CPMS 通知",
    width: WINDOW_WIDTH,
    height: WINDOW_HEIGHT,
    minWidth: WINDOW_WIDTH,
    minHeight: WINDOW_HEIGHT,
    maxWidth: WINDOW_WIDTH,
    maxHeight: WINDOW_HEIGHT,
    resizable: false,
    decorations: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    focus: false,
    visible: false,
    shadow: false,
    transparent: true,
  });

  try {
    await waitForWindowCreated(notificationWindow);
  } catch (error) {
    if (!isDuplicateWindowLabelError(error)) {
      throw error;
    }

    return (await WebviewWindow.getByLabel(DESKTOP_NOTIFICATION_WINDOW)) ?? notificationWindow;
  }

  await readyPromise;
  return notificationWindow;
}

async function waitForWindowCreated(notificationWindow: WebviewWindow) {
  return new Promise<void>((resolve, reject) => {
    void notificationWindow.once("tauri://created", () => resolve());
    void notificationWindow.once("tauri://error", (event) => {
      reject(new Error(String(event.payload)));
    });
  });
}

async function waitForEvent(eventName: string, timeoutMs: number) {
  return new Promise<void>((resolve) => {
    let unlisten: UnlistenFn | undefined;
    let done = false;
    const timer = window.setTimeout(finish, timeoutMs);

    function finish() {
      if (done) {
        return;
      }

      done = true;
      window.clearTimeout(timer);
      unlisten?.();
      resolve();
    }

    void once(eventName, finish)
      .then((nextUnlisten) => {
        if (done) {
          nextUnlisten();
          return;
        }

        unlisten = nextUnlisten;
      })
      .catch(finish);
  });
}

async function createNotificationAckWaiter(
  notificationId: string,
  timeoutMs: number,
): Promise<NotificationAckWaiter> {
  let unlisten: UnlistenFn | undefined;
  let received: DesktopNotificationAckPayload | undefined;
  let final: DesktopNotificationAckPayload | undefined;
  let done = false;
  let timer: number | undefined;
  let resolvePromise: (result: NotificationAckWaitResult) => void = () => undefined;
  const promise = new Promise<NotificationAckWaitResult>((resolve) => {
    resolvePromise = resolve;
  });

  function finish() {
    if (done) {
      return;
    }

    done = true;
    if (timer !== undefined) {
      window.clearTimeout(timer);
    }

    unlisten?.();
    resolvePromise({ received, final });
  }

  unlisten = await listen<DesktopNotificationAckPayload>(DESKTOP_NOTIFICATION_ACK_EVENT, (event) => {
    const payload = event.payload;

    if (payload.id !== notificationId) {
      return;
    }

    if (payload.stage === "received") {
      received = payload;
      return;
    }

    final = payload;
    received ??= payload;
    finish();
  });

  if (done) {
    unlisten();
  } else {
    timer = window.setTimeout(() => finish(), timeoutMs);
  }

  return { promise, cancel: finish };
}

async function positionNotificationWindow(notificationWindow: WebviewWindow) {
  const monitor = (await currentMonitor()) ?? (await primaryMonitor());

  if (!monitor) {
    return;
  }

  const scaleFactor = monitor.scaleFactor || 1;
  const width = WINDOW_WIDTH * scaleFactor;
  const height = WINDOW_HEIGHT * scaleFactor;
  const margin = WINDOW_MARGIN * scaleFactor;
  const x = monitor.workArea.position.x + monitor.workArea.size.width - width - margin;
  const y = monitor.workArea.position.y + monitor.workArea.size.height - height - margin;

  await notificationWindow.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
}

function pushStep(
  steps: DesktopNotificationDiagnosticStep[],
  name: string,
  status: DesktopNotificationDiagnosticStatus,
  startedAt: number,
  detail?: string,
) {
  const step: DesktopNotificationDiagnosticStep = {
    name,
    status,
    durationMs: Math.max(0, Math.round(now() - startedAt)),
  };

  if (detail) {
    step.detail = detail;
  }

  steps.push(step);
}

function now() {
  return typeof performance === "undefined" ? Date.now() : performance.now();
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function isDuplicateWindowLabelError(error: unknown) {
  return /window.*label.*already exists/i.test(errorMessage(error));
}
