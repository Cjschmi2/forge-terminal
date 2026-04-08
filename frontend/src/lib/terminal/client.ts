/**
 * WebSocket client that speaks the 21-byte wire frame protocol.
 *
 * Usage:
 *   const client = new WireClient({ url: 'wss://host/ws' });
 *   client.onFrame((frame) => { ... });
 *   await client.connect();
 *   client.send(someFrame);
 */

import { decodeFrames, encodeFrame, type TokenFrame } from './frame';
import { TOKEN } from './tokens';

export interface WireClientConfig {
  /** WebSocket URL (ws:// or wss://). */
  url: string;
  /** JWT added as `?token=` query parameter on the WS upgrade request. */
  token?: string;
  /** Whether to automatically reconnect on disconnect (default: true). */
  reconnect?: boolean;
  /** Max consecutive reconnect attempts before giving up (default: 10). */
  maxReconnectAttempts?: number;
  /** Base delay in ms between reconnect attempts, doubled each time (default: 1000). */
  reconnectBaseDelay?: number;
}

type FrameCallback = (frame: TokenFrame) => void;
type LifecycleCallback = () => void;

export class WireClient {
  private readonly config: Required<WireClientConfig>;
  private ws: WebSocket | null = null;
  private frameListeners: Set<FrameCallback> = new Set();
  private connectListeners: Set<LifecycleCallback> = new Set();
  private disconnectListeners: Set<LifecycleCallback> = new Set();
  private reconnectAttempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionalClose = false;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;

  constructor(config: WireClientConfig) {
    this.config = {
      url: config.url,
      token: config.token ?? '',
      reconnect: config.reconnect ?? true,
      maxReconnectAttempts: config.maxReconnectAttempts ?? 10,
      reconnectBaseDelay: config.reconnectBaseDelay ?? 1000,
    };
  }

  /** Open the WebSocket connection. Resolves when the socket is open. */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.intentionalClose = false;
      this.clearReconnectTimer();

      const url = this.buildUrl();
      const ws = new WebSocket(url);
      ws.binaryType = 'arraybuffer';

      ws.addEventListener('open', () => {
        this.ws = ws;
        this.reconnectAttempt = 0;
        this.startHeartbeat();
        this.connectListeners.forEach((cb) => cb());
        resolve();
      });

      ws.addEventListener('message', (event) => {
        if (!(event.data instanceof ArrayBuffer)) return;
        const frames = decodeFrames(event.data);
        for (const frame of frames) {
          this.frameListeners.forEach((cb) => cb(frame));
        }
      });

      ws.addEventListener('close', () => {
        this.stopHeartbeat();
        this.ws = null;
        this.disconnectListeners.forEach((cb) => cb());
        if (!this.intentionalClose && this.config.reconnect) {
          this.scheduleReconnect();
        }
      });

      ws.addEventListener('error', (event) => {
        if (!this.ws) {
          reject(new Error('WebSocket connection failed'));
        }
        // Errors on an established connection are followed by a close event,
        // which triggers the reconnect logic above.
      });
    });
  }

  /** Gracefully close the WebSocket. No reconnect will be attempted. */
  disconnect(): void {
    this.intentionalClose = true;
    this.clearReconnectTimer();
    this.stopHeartbeat();
    if (this.ws) {
      this.ws.close(1000, 'client disconnect');
      this.ws = null;
    }
  }

  /** Send a wire frame over the open socket. Silently drops if not connected. */
  send(frame: TokenFrame): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    this.ws.send(encodeFrame(frame));
  }

  /** Subscribe to decoded frames. Returns an unsubscribe function. */
  onFrame(callback: FrameCallback): () => void {
    this.frameListeners.add(callback);
    return () => {
      this.frameListeners.delete(callback);
    };
  }

  /** Subscribe to successful connection events. Returns an unsubscribe function. */
  onConnect(callback: LifecycleCallback): () => void {
    this.connectListeners.add(callback);
    return () => {
      this.connectListeners.delete(callback);
    };
  }

  /** Subscribe to disconnection events. Returns an unsubscribe function. */
  onDisconnect(callback: LifecycleCallback): () => void {
    this.disconnectListeners.add(callback);
    return () => {
      this.disconnectListeners.delete(callback);
    };
  }

  /** Whether the WebSocket is currently in the OPEN state. */
  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  // ---- private ----

  private buildUrl(): string {
    const base = this.config.url;
    if (!this.config.token) return base;
    const separator = base.includes('?') ? '&' : '?';
    return `${base}${separator}token=${encodeURIComponent(this.config.token)}`;
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempt >= this.config.maxReconnectAttempts) return;
    const delay = this.config.reconnectBaseDelay * Math.pow(2, this.reconnectAttempt);
    this.reconnectAttempt++;
    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(() => {
        // connect() rejection is expected if the server is still down;
        // the close handler will schedule the next attempt.
      });
    }, delay);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      this.send({
        tokenType: TOKEN.SESSION_HEARTBEAT,
        streamId: 0,
        sequence: 0,
        timestampNs: BigInt(Date.now()) * 1_000_000n,
        data: new Uint8Array(0),
      });
    }, 30_000);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer !== null) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }
}
