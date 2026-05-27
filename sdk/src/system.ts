// System SDK — storage / dialog / notification / system (3-17)

// 命令发送（由运行时注入）
type SendFn = (method: string, params: Record<string, unknown>) => Promise<unknown>;
let _send: SendFn = async () => {};

export function _setSend(fn: SendFn): void {
  _send = fn;
}

// ── KV Storage ──
export const storage = {
  async get(key: string): Promise<unknown> {
    return _send("storage.get", { key });
  },
  async set(key: string, value: unknown): Promise<void> {
    await _send("storage.set", { key, value });
  },
  async delete(key: string): Promise<void> {
    await _send("storage.delete", { key });
  },
};

// ── Dialog ──
export const dialog = {
  async alert(message: string): Promise<void> {
    await _send("dialog.alert", { message });
  },
  async confirm(message: string): Promise<boolean> {
    const result = await _send("dialog.confirm", { message });
    return result as boolean;
  },
  async prompt(message: string, defaultValue?: string): Promise<string | null> {
    const result = await _send("dialog.prompt", { message, defaultValue });
    return result as string | null;
  },
};

// ── Notification ──
export const notification = {
  show(title: string, body?: string): void {
    _send("notification.show", { title, body });
  },
};

// ── System ──
export const system = {
  async launch(path: string): Promise<void> {
    await _send("system.launch", { path });
  },
  async memory(): Promise<{ heapUsed: number; heapTotal: number; cpuMs: number }> {
    const result = await _send("system.memory", {});
    return result as { heapUsed: number; heapTotal: number; cpuMs: number };
  },
};
