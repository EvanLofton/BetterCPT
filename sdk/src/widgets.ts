// Widget SDK — 命令式 API，所有操作序列化为 JSON-RPC 经 Scheduler 发送

// ── 类型 ──
export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface WidgetOptions {
  blur?: boolean;
  radius?: number;
  width: number;
  height: number;
  color?: string;
}

export interface IconOptions {
  src: string;
  size: number;
}

export interface TextOptions {
  size?: number;
  weight?: "normal" | "bold" | number;
  color?: string;
}

export interface MouseEvent {
  x: number;
  y: number;
}

// ── Widget 句柄 ──
let _nextId = 0;
function nextId(): string {
  return `widget_${++_nextId}`;
}

// 命令队列（由 SDK 运行时注入实际发送函数）
type SendFn = (method: string, params: Record<string, unknown>) => void;
let _send: SendFn = (_method, _params) => {
  // Phase 3: Deno ops 注入后替换
};

export function _setSend(fn: SendFn): void {
  _send = fn;
}

// ── Widget 类 ──
export class Widget {
  id: string;
  private _children: Widget[] = [];

  constructor(id: string) {
    this.id = id;
  }

  // 层级操作 (3-5)
  addChild(child: Widget): void {
    this._children.push(child);
    _send("addChild", { parentId: this.id, childId: child.id });
  }

  removeChild(child: Widget): void {
    this._children = this._children.filter(c => c.id !== child.id);
    _send("removeChild", { parentId: this.id, childId: child.id });
  }

  // 动画 (3-4)
  setScale(scale: number, duration: number): void {
    _send("setScale", { id: this.id, scale, duration });
  }

  setOpacity(opacity: number, duration: number): void {
    _send("setOpacity", { id: this.id, opacity, duration });
  }

  setBlur(enabled: boolean): void {
    _send("setBlur", { id: this.id, enabled });
  }

  // 事件 (3-5, 3-6)
  onHover(callback: () => void): void {
    _send("onHover", { id: this.id });
    // callback 注册由 Host 侧管理，事件触发时 Host 回调到对应 Isolate
  }

  onLeave(callback: () => void): void {
    _send("onLeave", { id: this.id });
  }

  onClick(callback: () => void): void {
    _send("onClick", { id: this.id });
  }

  onMouseDown(callback: (e: MouseEvent) => void): void {
    _send("onMouseDown", { id: this.id });
  }

  onMouseMove(callback: (e: MouseEvent) => void): void {
    _send("onMouseMove", { id: this.id });
  }

  onMouseUp(callback: (e: MouseEvent) => void): void {
    _send("onMouseUp", { id: this.id });
  }

  // Z-Order (3-7)
  raise(): void {
    _send("raise", { id: this.id });
  }

  lower(): void {
    _send("lower", { id: this.id });
  }
}

// ── 创建函数 (3-3) ──
export function createRect(options: WidgetOptions): Widget {
  const id = nextId();
  _send("createRect", { id, ...options });
  return new Widget(id);
}

export function createIcon(options: IconOptions): Widget {
  const id = nextId();
  _send("createIcon", { id, ...options });
  return new Widget(id);
}

export function createText(text: string, options?: TextOptions): Widget {
  const id = nextId();
  _send("createText", { id, text, ...options });
  return new Widget(id);
}
