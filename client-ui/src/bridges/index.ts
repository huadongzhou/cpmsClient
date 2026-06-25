/**
 * 桥接层统一出口（收口三类桥）：
 * - 客户端 Tauri 事件桥：useClient{Event,Log,Notification,Runtime}Bridge；
 * - iframe postMessage 桥：useIframePayloadBridge + hubBridge（父窗口侧 RPC/事件转发）；
 * - 桌面通知窗口桥：useDesktopNotificationBridge。
 */
export { useClientEventBridge } from "./useClientEventBridge";
export { useClientLogBridge } from "./useClientLogBridge";
export { useClientNotificationBridge } from "./useClientNotificationBridge";
export { useClientRuntimeBridge } from "./useClientRuntimeBridge";
export { useDesktopNotificationBridge } from "./useDesktopNotificationBridge";
export { useIframePayloadBridge } from "./useIframePayloadBridge";
export type { IframePayloadBridgeResult } from "./useIframePayloadBridge";
export { startHubClientMessageBridge } from "./hubBridge";
