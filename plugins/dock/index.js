// @bettercpt/dock v0.2.0 — Mac 风格 Dock

var cfg = {
  bgColor: "rgba(20, 20, 20, 0.70)", bgOpacity: 30, bgRadius: 16,
  iconSize: 48, iconTopMargin: 10, iconLeftMargin: 16, iconGap: 16,
  dockVisible: true,
};
if (typeof config !== "undefined") {
  for (var k in cfg) { if (config[k] !== undefined) cfg[k] = config[k]; }
}

var SW = system.screenWidth, SH = system.screenHeight;
console.log("[Dock] screen: " + SW + "x" + SH);

var MAX_ICONS = 12, icons = [];
var DOCK_W = MAX_ICONS * cfg.iconSize + (MAX_ICONS - 1) * cfg.iconGap + cfg.iconLeftMargin * 2;
var DOCK_H = cfg.iconSize + cfg.iconTopMargin * 2 + 10; // extra space for dots
var DOCK_X = Math.floor((SW - DOCK_W) / 2), DOCK_Y = SH - DOCK_H - 60;

var dockBg = createRect({
  width: DOCK_W, height: DOCK_H,
  x: DOCK_X, y: DOCK_Y,
  radius: cfg.bgRadius, color: cfg.bgColor, blur: false,
});

// ── 添加图标 ──
function addIcon(filePath) {
  var idx = icons.length;
  var localX = cfg.iconLeftMargin + idx * (cfg.iconSize + cfg.iconGap);
  var localY = cfg.iconTopMargin;
  // 提取文件名（兼容 / 和 \\ 路径分隔符）
  var name = filePath.replace(/\\/g, "/").split("/").pop().replace(/\.lnk$/i, "");

  // 图标
  var icon = createIcon({
    src: filePath, size: cfg.iconSize,
    x: localX, y: localY, containerId: dockBg.id,
  });

  // 指示点
  var dotX = localX + Math.floor(cfg.iconSize / 2) - 3;
  var dotY = cfg.iconTopMargin + cfg.iconSize + 4;
  var dot = createRect({
    width: 6, height: 6, x: dotX, y: dotY,
    radius: 3, color: "rgba(255, 255, 255, 0.5)",
    containerId: dockBg.id,
  });

  icon.onHover(function() { icon.setScale(1.25, 200); });
  icon.onLeave(function() { icon.setScale(1.0, 250); });

  // ── click: 启动 ──
  icon.onClick(function() {
    try { system.launch(filePath); } catch (e) {}
  });

  icons.push({ path: filePath, name: name, widget: icon, dot: dot });
  console.log("[Dock] +" + name);
}

// ── 文件拖放 ──
dockBg.onDrop(function(paths) {
  for (var i = 0; i < paths.length; i++) {
    if (icons.length >= MAX_ICONS) { console.log("[Dock] limit " + MAX_ICONS); break; }
    addIcon(paths[i]);
  }
});

onConfigChange = function() {
  // 运行时实时生效：重新读取 config 中所有设定
  for (var k in cfg) { if (config[k] !== undefined) cfg[k] = config[k]; }
  // 颜色/透明度/圆角 → Dashboard 通过 sendWidgetCmd setColor 实时生效
  // 图标布局 (iconSize/iconTopMargin/iconLeftMargin/iconGap) → 下次启动生效
  //   (v1 SDK 无 setGeometry，无法运行时移动已创建 widget)
  console.log("[Dock] config updated, layout applies on restart");
};
console.log("[Dock] ready, max " + MAX_ICONS);
