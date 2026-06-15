import { isTauri } from "@tauri-apps/api/core";
import { emitTo, once, type UnlistenFn } from "@tauri-apps/api/event";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { currentMonitor, primaryMonitor } from "@tauri-apps/api/window";
import type { AppNotification } from "@/types/app/notification";

export const DESKTOP_NOTIFICATION_WINDOW = "notification";
export const DESKTOP_NOTIFICATION_PUSH_EVENT = "desktop-notification:push";
export const DESKTOP_NOTIFICATION_READY_EVENT = "desktop-notification:ready";

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

async function ensureNotificationWindow() {
  const existingWindow = await WebviewWindow.getByLabel(DESKTOP_NOTIFICATION_WINDOW);

  if (existingWindow) {
    return existingWindow;
  }

  notificationWindowPromise ??= createNotificationWindow().catch((error) => {
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
    shadow: true,
  });

  await waitForWindowCreated(notificationWindow);
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
