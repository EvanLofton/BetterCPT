#include "WidgetManager.h"
#include "RoundedWidget.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QApplication>
#include <QScreen>
#include <QLabel>
#include <QPixmap>
#include <QRegularExpression>
#include <QPropertyAnimation>
#include <QGraphicsOpacityEffect>
#include <QTimer>
#include <QEvent>
#include <QMouseEvent>
#include <QDebug>
#include <QFileIconProvider>
#include <QFileInfo>

#include <Windows.h>
#include <shlobj.h>
#include <shobjidl.h>

// ── 事件过滤器：捕获 widget 鼠标事件，写成 JSON-RPC notification 到 stdout ──
class WidgetEventFilter : public QObject {
    QString m_widgetId;
public:
    explicit WidgetEventFilter(const QString &widgetId, QObject *parent = nullptr)
        : QObject(parent), m_widgetId(widgetId) {}

    bool eventFilter(QObject *, QEvent *event) override {
        QByteArray notif;
        if (event->type() == QEvent::Enter) {
            notif = QString("{\"jsonrpc\":\"2.0\",\"method\":\"onHover\",\"params\":{\"widgetId\":\"%1\"}}\n")
                .arg(m_widgetId).toUtf8();
        } else if (event->type() == QEvent::Leave) {
            notif = QString("{\"jsonrpc\":\"2.0\",\"method\":\"onLeave\",\"params\":{\"widgetId\":\"%1\"}}\n")
                .arg(m_widgetId).toUtf8();
        } else if (event->type() == QEvent::MouseButtonPress) {
            notif = QString("{\"jsonrpc\":\"2.0\",\"method\":\"onClick\",\"params\":{\"widgetId\":\"%1\"}}\n")
                .arg(m_widgetId).toUtf8();
            // 原生窗口拖动：仅顶层窗口触发，子 Widget 跳过
            if (auto *w = qobject_cast<QWidget*>(parent())) {
                if (w->isWindow()) {
                    if (HWND hwnd = reinterpret_cast<HWND>(w->winId())) {
                        ReleaseCapture();
                        SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION, 0);
                    }
                }
            }
        }
        if (!notif.isEmpty()) {
            fwrite(notif.constData(), 1, notif.size(), stdout);
            fflush(stdout);
        }
        return false; // 继续传递事件，不影响正常行为
    }
};
// ── end event filter ──

// ── Overlay flags ──
static void applyOverlayFlags(QWidget *w) {
    w->setWindowFlags(
        Qt::FramelessWindowHint
        | Qt::WindowStaysOnTopHint
    );
}

// ── 任务栏隐藏 (L2 fix) ──
// Phase 3: SetWindowLongPtrW + WS_EX_TOOLWINDOW 在时机不对时崩溃。
// 修复方案：QTimer::singleShot(0) 延迟到 widget 完全创建后执行，
// 加 IsWindow 校验，并用 SetWindowPos + SWP_FRAMECHANGED 同步。
static void hideFromTaskbar(QWidget *w) {
    HWND hwnd = reinterpret_cast<HWND>(w->winId());
    if (!hwnd || !IsWindow(hwnd)) return;

    LONG_PTR exstyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if (exstyle == 0) return;

    // 移除 WS_EX_APPWINDOW（任务栏显示）+ 添加 WS_EX_TOOLWINDOW（隐藏）
    exstyle &= ~WS_EX_APPWINDOW;
    exstyle |= WS_EX_TOOLWINDOW;

    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, exstyle);
    SetWindowPos(hwnd, nullptr, 0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE);
}

// ── Constructor ──
WidgetManager::WidgetManager(QObject *parent) : QObject(parent) {}

// ── Dispatch ──
QJsonObject WidgetManager::handleCommand(const QString &method, const QJsonObject &params) {
    if (method == "createRect")   return createRect(params);
    if (method == "createIcon")   return createIcon(params);
    if (method == "createText")   return createText(params);
    if (method == "setScale")     return setScale(params);
    if (method == "setOpacity")   return setOpacity(params);
    if (method == "addChild")     return addChild(params);
    if (method == "removeChild")  return removeChild(params);
    if (method == "raise")        return raiseWidget(params);
    if (method == "lower")        return lowerWidget(params);
    if (method == "removeWidget")  return removeWidget(params);
    if (method == "setColor")     return setWidgetColor(params);
    if (method == "ping")         { QJsonObject r; r["status"] = "ok"; return r; }

    QJsonObject err;
    err["error"] = QJsonObject{{"code", -32601}, {"message", QString("Method not found: %1").arg(method)}};
    return err;
}

// ── Helpers ──
QWidget* WidgetManager::findWidget(const QString &id) {
    auto it = m_widgets.find(id);
    return it != m_widgets.end() ? it.value().widget : nullptr;
}

QRect WidgetManager::scaledGeometry(const WidgetEntry &e, double scale) const {
    const QRect &b = e.baseGeometry;
    double dw = b.width()  * (scale - 1.0);
    double dh = b.height() * (scale - 1.0);
    return QRect(
        static_cast<int>(b.x() - dw / 2.0),
        static_cast<int>(b.y() - dh / 2.0),
        static_cast<int>(b.width()  * scale),
        static_cast<int>(b.height() * scale)
    );
}

void WidgetManager::updateContainerMask() {
    // RoundedWidget 通过 QPainter + WA_TranslucentBackground 处理圆角与透明度，无需 mask
}

// ── 颜色解析：支持 "#RRGGBB" / "rgba(r,g,b,a)" / "rgb(r,g,b)" ──
static QColor parseColor(const QString &s) {
    QRegularExpression rgba(R"(rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\))");
    auto m = rgba.match(s);
    if (m.hasMatch()) {
        return QColor(m.captured(1).toInt(), m.captured(2).toInt(),
                      m.captured(3).toInt(), static_cast<int>(m.captured(4).toDouble() * 255));
    }
    QRegularExpression rgb(R"(rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\))");
    m = rgb.match(s);
    if (m.hasMatch()) {
        return QColor(m.captured(1).toInt(), m.captured(2).toInt(), m.captured(3).toInt());
    }
    return QColor(s); // #RRGGBB 或颜色名
}

// ── .lnk 解析：提取快捷方式目标 exe 路径 ──
static QString resolveLnkTarget(const QString &lnkPath) {
    QString result;
    IShellLinkW *psl = nullptr;
    IPersistFile *ppf = nullptr;
    HRESULT hr = CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER,
                                   IID_IShellLinkW, (void **)&psl);
    if (FAILED(hr)) return result;
    hr = psl->QueryInterface(IID_IPersistFile, (void **)&ppf);
    if (SUCCEEDED(hr)) {
        std::wstring ws = lnkPath.toStdWString();
        hr = ppf->Load(ws.c_str(), STGM_READ);
        if (SUCCEEDED(hr)) {
            wchar_t target[MAX_PATH];
            if (SUCCEEDED(psl->GetPath(target, MAX_PATH, nullptr, SLGP_RAWPATH))) {
                result = QString::fromWCharArray(target);
            }
        }
    }
    if (ppf) ppf->Release();
    if (psl) psl->Release();
    return result;
}

// ── 提取 exe 内嵌图标 ──
static QIcon extractFileIcon(const QString &filePath) {
    QFileIconProvider provider;
    QFileInfo fi(filePath);
    if (fi.exists()) {
        return provider.icon(fi);
    }
    return QIcon();
}

// ── 3-3: createRect ──
QJsonObject WidgetManager::createRect(const QJsonObject &p) {
    QString id = p.value("id").toString();
    int x = p.value("x").toInt(0), y = p.value("y").toInt(0);
    int w = p.value("w").toInt(100), h = p.value("h").toInt(100);
    int radius = p.value("radius").toInt(0);
    QString color = p.value("color").toString("rgba(30,30,30,200)");
    bool blur = p.value("blur").toBool(false);
    QString containerId = p.value("containerId").toString();

    // 幂等：id 已存在则先删除旧 widget
    auto existing = m_widgets.find(id);
    if (existing != m_widgets.end()) {
        if (existing.value().widget) delete existing.value().widget;
        m_widgets.erase(existing);
    }

    WidgetEntry entry;

    // 有父容器 → 创建为 Qt 子控件（本地坐标、无独立窗口标志）
    if (!containerId.isEmpty()) {
        QWidget *parent = findWidget(containerId);
        if (parent) {
            QWidget *child = new QWidget(parent);
            child->setGeometry(x, y, w, h);
            child->setStyleSheet(QString("background: %1; border-radius: %2px;").arg(color).arg(radius));
            child->setAttribute(Qt::WA_Hover, true);
            child->installEventFilter(new WidgetEventFilter(id, child));
            child->show();
            entry.widget = child;
            entry.baseGeometry = QRect(x, y, w, h);
            entry.radius = radius;
            m_widgets[id] = entry;
            qDebug() << "[WidgetManager] createRect(child):" << id << x << y << w << "x" << h << "container=" << containerId;
            QJsonObject r; r["status"] = "ok"; r["widgetId"] = id; return r;
        }
    }

    // 无父容器 → 独立顶层窗口
    RoundedWidget *rect = new RoundedWidget();
    rect->setGeometry(x, y, w, h);
    rect->setBgColor(parseColor(color));
    rect->setRadius(radius);
    applyOverlayFlags(rect);
    rect->setAttribute(Qt::WA_Hover, true);
    rect->installEventFilter(new WidgetEventFilter(id, rect));
    // 文件拖放 → JSON-RPC 通知到 stdout
    QObject::connect(rect, &RoundedWidget::fileDropped, [id](const QStringList &paths) {
        QJsonArray arr;
        for (const auto &p : paths) arr.append(p);
        QJsonObject params;
        params["widgetId"] = id;
        params["paths"] = arr;
        QJsonObject notif;
        notif["jsonrpc"] = QString("2.0");
        notif["method"] = QString("onDrop");
        notif["params"] = params;
        QByteArray line = QJsonDocument(notif).toJson(QJsonDocument::Compact) + "\n";
        fwrite(line.constData(), 1, line.size(), stdout);
        fflush(stdout);
    });
    rect->show();
    QTimer::singleShot(0, [rect] {
        hideFromTaskbar(rect);
        rect->enableFileDrop();
    });

    entry.widget = rect;
    entry.baseGeometry = QRect(x, y, w, h);
    entry.radius = radius;
    m_widgets[id] = entry;
    qDebug() << "[WidgetManager] createRect:" << id << x << y << w << "x" << h << "r=" << radius;
    QJsonObject r; r["status"] = "ok"; r["widgetId"] = id; return r;
}

// ── 3-3: createIcon ──
QJsonObject WidgetManager::createIcon(const QJsonObject &p) {
    QString id = p.value("id").toString();
    int x = p.value("x").toInt(0), y = p.value("y").toInt(0);
    int size = p.value("size").toInt(48);
    QString src = p.value("src").toString();
    QString containerId = p.value("containerId").toString();

    // 幂等
    auto existingIcon = m_widgets.find(id);
    if (existingIcon != m_widgets.end()) {
        if (existingIcon.value().widget) delete existingIcon.value().widget;
        m_widgets.erase(existingIcon);
    }

    // 解析图标源：.lnk → exe 路径
    QString iconSrc = src;
    if (src.endsWith(".lnk", Qt::CaseInsensitive)) {
        QString target = resolveLnkTarget(src);
        if (!target.isEmpty()) {
            iconSrc = target;
            qDebug() << "[createIcon] .lnk resolved:" << src << "→" << target;
        }
    }

    WidgetEntry entry;

    QLabel *label = nullptr;
    QWidget *parent = containerId.isEmpty() ? nullptr : findWidget(containerId);
    if (parent) {
        label = new QLabel(parent);          // Qt 子控件
        label->setGeometry(x, y, size, size);
    } else {
        label = new QLabel();                 // 独立顶层窗口
        label->setGeometry(x, y, size, size);
        applyOverlayFlags(label);
    }
    label->setAlignment(Qt::AlignCenter);

    // 尝试提取图标
    QIcon fileIcon = extractFileIcon(iconSrc);
    if (!fileIcon.isNull()) {
        label->setPixmap(fileIcon.pixmap(size, size));
    } else {
        // fallback：纯色方块
        label->setStyleSheet(QString("background: rgba(255,255,255,180); border-radius: %1px;").arg(size / 4));
    }

    label->setAttribute(Qt::WA_Hover, true);
    label->installEventFilter(new WidgetEventFilter(id, label));
    label->show();

    if (!parent) {
        QTimer::singleShot(0, [label] { hideFromTaskbar(label); });
    }

    entry.widget = label;
    entry.baseGeometry = QRect(x, y, size, size);
    m_widgets[id] = entry;
    QTimer::singleShot(0, [this] { updateContainerMask(); });
    qDebug() << "[WidgetManager] createIcon:" << id << size << src << "→" << iconSrc;
    QJsonObject r; r["status"] = "ok"; r["widgetId"] = id; return r;
}

// ── 3-3: createText ──
QJsonObject WidgetManager::createText(const QJsonObject &p) {
    QString id = p.value("id").toString();
    QString text = p.value("text").toString();
    int x = p.value("x").toInt(0), y = p.value("y").toInt(0);
    int fontSize = p.value("size").toInt(16);
    QString color = p.value("color").toString("#ffffff");
    QString weight = p.value("weight").toString("normal");

    // 幂等
    auto existingText = m_widgets.find(id);
    if (existingText != m_widgets.end()) {
        if (existingText.value().widget) delete existingText.value().widget;
        m_widgets.erase(existingText);
    }

    QLabel *label = new QLabel(text);
    applyOverlayFlags(label);
    label->setGeometry(x, y, 200, 40);
    label->setStyleSheet(QString("color: %1; font-size: %2px; font-weight: %3; background: transparent;").arg(color).arg(fontSize).arg(weight));
    label->adjustSize();
    label->setAttribute(Qt::WA_Hover, true);
    label->installEventFilter(new WidgetEventFilter(id, label));
    label->show();
    QTimer::singleShot(0, [label] { hideFromTaskbar(label); });

    QRect geo = label->geometry();
    WidgetEntry entry;
    entry.widget = label;
    entry.baseGeometry = geo;
    m_widgets[id] = entry;
    QTimer::singleShot(0, [this] { updateContainerMask(); });
    qDebug() << "[WidgetManager] createText:" << id << text;
    QJsonObject r; r["status"] = "ok"; r["widgetId"] = id; return r;
}

// ── 3-4: setScale ──
QJsonObject WidgetManager::setScale(const QJsonObject &p) {
    QString id = p.value("id").toString();
    double scale = p.value("scale").toDouble(1.0);
    int duration = p.value("duration").toInt(200);
    auto it = m_widgets.find(id);
    if (it == m_widgets.end()) {
        QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", QString("Widget not found: %1").arg(id)}}; return err;
    }
    WidgetEntry &e = it.value();
    QRect target = scaledGeometry(e, scale);
    auto *anim = new QPropertyAnimation(e.widget, "geometry");
    anim->setDuration(duration);
    anim->setStartValue(e.widget->geometry());
    anim->setEndValue(target);
    anim->setEasingCurve(QEasingCurve::OutBack);
    anim->start(QAbstractAnimation::DeleteWhenStopped);
    e.currentScale = scale;
    QTimer::singleShot(0, [this] { updateContainerMask(); });
    // qDebug() << "[WidgetManager] setScale:" << id << "→" << scale;
    QJsonObject r; r["status"] = "ok"; r["widgetId"] = id; return r;
}

// ── 3-4: setOpacity ──
QJsonObject WidgetManager::setOpacity(const QJsonObject &p) {
    QString id = p.value("id").toString();
    double opacity = p.value("opacity").toDouble(1.0);
    int duration = p.value("duration").toInt(150);
    auto it = m_widgets.find(id);
    if (it == m_widgets.end()) {
        QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", QString("Widget not found: %1").arg(id)}}; return err;
    }
    WidgetEntry &e = it.value();
    QWidget *w = e.widget;
    auto *effect = qobject_cast<QGraphicsOpacityEffect*>(w->graphicsEffect());
    if (!effect) { effect = new QGraphicsOpacityEffect(w); effect->setOpacity(e.currentOpacity); w->setGraphicsEffect(effect); }
    auto *anim = new QPropertyAnimation(effect, "opacity");
    anim->setDuration(duration);
    anim->setStartValue(effect->opacity());
    anim->setEndValue(opacity);
    anim->setEasingCurve(QEasingCurve::OutBack);
    anim->start(QAbstractAnimation::DeleteWhenStopped);
    e.currentOpacity = opacity;
    qDebug() << "[WidgetManager] setOpacity:" << id << "→" << opacity;
    QJsonObject r; r["status"] = "ok"; r["widgetId"] = id; return r;
}

// ── 3-5: addChild / removeChild ──
QJsonObject WidgetManager::addChild(const QJsonObject &p) {
    QWidget *parent = findWidget(p.value("parentId").toString());
    QWidget *child  = findWidget(p.value("childId").toString());
    if (!parent || !child) { QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", "Parent or child not found"}}; return err; }
    // 保持 child 为独立顶层窗口，只调层级
    child->raise();
    child->show();
    qDebug() << "[WidgetManager] addChild:" << p.value("childId").toString();
    QJsonObject r; r["status"] = "ok"; return r;
}
QJsonObject WidgetManager::removeChild(const QJsonObject &p) {
    QWidget *child = findWidget(p.value("childId").toString());
    if (!child) { QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", "Widget not found"}}; return err; }
    child->setParent(nullptr);
    qDebug() << "[WidgetManager] removeChild:" << p.value("childId").toString();
    QJsonObject r; r["status"] = "ok"; return r;
}

// ── 3-7: raise / lower ──
QJsonObject WidgetManager::raiseWidget(const QJsonObject &p) {
    QWidget *w = findWidget(p.value("id").toString());
    if (!w) { QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", "Widget not found"}}; return err; }
    w->raise(); qDebug() << "[WidgetManager] raise:" << p.value("id").toString();
    QJsonObject r; r["status"] = "ok"; return r;
}
QJsonObject WidgetManager::lowerWidget(const QJsonObject &p) {
    QWidget *w = findWidget(p.value("id").toString());
    if (!w) { QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", "Widget not found"}}; return err; }
    w->lower(); qDebug() << "[WidgetManager] lower:" << p.value("id").toString();
    QJsonObject r; r["status"] = "ok"; return r;
}

// ── removeWidget: 删除 widget 并从映射表移除 ──
QJsonObject WidgetManager::removeWidget(const QJsonObject &p) {
    QString id = p.value("id").toString();
    auto it = m_widgets.find(id);
    if (it == m_widgets.end()) {
        QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", "Widget not found"}}; return err;
    }
    if (it.value().widget) {
        qDebug() << "[removeWidget] HIDING:" << id;
        it.value().widget->hide();
        qDebug() << "[removeWidget] HIDDEN:" << id << "visible=" << it.value().widget->isVisible();
        it.value().widget->deleteLater();
        qDebug() << "[removeWidget] DELETED:" << id;
    }
    m_widgets.erase(it);
    QTimer::singleShot(0, [this] { updateContainerMask(); });
    qDebug() << "[WidgetManager] removeWidget:" << id;
    QJsonObject r; r["status"] = "ok"; return r;
}

// ── setWidgetColor: 运行时修改背景色、透明度和圆角 ──
QJsonObject WidgetManager::setWidgetColor(const QJsonObject &p) {
    QString id = p.value("id").toString();
    QString color = p.value("color").toString();
    int radius = p.value("radius").toInt(-1);
    auto it = m_widgets.find(id);
    if (it == m_widgets.end()) {
        QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", "Widget not found"}}; return err;
    }
    QWidget *w = it.value().widget;
    if (!w) {
        QJsonObject err; err["error"] = QJsonObject{{"code", -32001}, {"message", "Widget is null"}}; return err;
    }

    QColor qc = parseColor(color);
    // 如果是 RoundedWidget，直接设置属性（QPainter 渲染 + alpha 通道）
    if (auto *rw = qobject_cast<RoundedWidget *>(w)) {
        rw->setBgColor(qc);
        if (radius >= 0) { rw->setRadius(radius); it.value().radius = radius; }
    } else {
        // icon/text label 的 stylesheet fallback
        w->setStyleSheet(QString("background: %1; border-radius: %2px;").arg(color).arg(radius >= 0 ? radius : 0));
    }
    qDebug() << "[WidgetManager] setWidgetColor:" << id << color << "radius:" << radius;
    QJsonObject r; r["status"] = "ok"; return r;
}
