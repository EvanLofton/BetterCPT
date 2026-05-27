#include "WidgetManager.h"
#include <QPropertyAnimation>
#include <QGraphicsOpacityEffect>
#include <QRegion>
#include <QDebug>

WidgetManager::WidgetManager(QWidget *container, QObject *parent)
    : QObject(parent), m_container(container)
{
}

QJsonObject WidgetManager::handleCommand(const QString &method, const QJsonObject &params) {
    if (method == "createRect")   return createRect(params);
    if (method == "createIcon")   return createIcon(params);
    if (method == "createText")   return createText(params);
    if (method == "setScale")     return setScale(params);
    if (method == "setOpacity")   return setOpacity(params);

    QJsonObject result;
    QJsonObject err;
    err["code"] = -32601;
    err["message"] = QString("Method not found: %1").arg(method);
    result["error"] = err;
    return result;
}

QJsonObject WidgetManager::createRect(const QJsonObject &params) {
    QString id = params.value("id").toString();
    if (id.isEmpty()) {
        QJsonObject result;
        QJsonObject err;
        err["code"] = -32602;
        err["message"] = QString("Invalid params: id is required");
        result["error"] = err;
        return result;
    }

    int x = params.value("x").toInt(0);
    int y = params.value("y").toInt(0);
    int w = params.value("w").toInt(100);
    int h = params.value("h").toInt(100);
    int radius = params.value("radius").toInt(0);
    QString color = params.value("color").toString("rgba(60,60,60,220)");

    QWidget *rect = new QWidget(m_container);
    rect->setGeometry(x, y, w, h);
    rect->setStyleSheet(
        QString("background: %1; border-radius: %2px;").arg(color).arg(radius)
    );
    rect->show();

    WidgetEntry entry;
    entry.widget = rect;
    entry.baseGeometry = QRect(x, y, w, h);
    entry.currentScale = 1.0;
    entry.currentOpacity = 1.0;
    m_widgets[id] = entry;
    updateClickMask();

    qDebug() << "[WidgetManager] createRect:" << id << "at" << x << y << w << "x" << h;

    QJsonObject result;
    result["status"] = "ok";
    result["widgetId"] = id;
    return result;
}

QJsonObject WidgetManager::createIcon(const QJsonObject &params) {
    QString id = params.value("id").toString();
    if (id.isEmpty()) {
        QJsonObject result;
        QJsonObject err;
        err["code"] = -32602;
        err["message"] = QString("Invalid params: id is required");
        result["error"] = err;
        return result;
    }

    int x = params.value("x").toInt(0);
    int y = params.value("y").toInt(0);
    int size = params.value("size").toInt(48);
    QString src = params.value("src").toString();

    QWidget *icon = new QWidget(m_container);
    icon->setGeometry(x, y, size, size);
    icon->setStyleSheet(
        QString("background: rgba(255,255,255,180); border-radius: %1px;").arg(size / 4)
    );
    icon->show();

    WidgetEntry entry;
    entry.widget = icon;
    entry.baseGeometry = QRect(x, y, size, size);
    entry.currentScale = 1.0;
    entry.currentOpacity = 1.0;
    m_widgets[id] = entry;
    updateClickMask();

    qDebug() << "[WidgetManager] createIcon:" << id << "size" << size << "src" << src;

    QJsonObject result;
    result["status"] = "ok";
    result["widgetId"] = id;
    return result;
}

QJsonObject WidgetManager::createText(const QJsonObject &params) {
    QString id = params.value("id").toString();
    if (id.isEmpty()) {
        QJsonObject result;
        QJsonObject err;
        err["code"] = -32602;
        err["message"] = QString("Invalid params: id is required");
        result["error"] = err;
        return result;
    }
    QJsonObject result;
    result["status"] = "ok";
    result["widgetId"] = id;
    result["warning"] = QString("createText not yet implemented in spike");
    return result;
}

QRect WidgetManager::scaledGeometry(const WidgetEntry &entry, double scale) const {
    const QRect &orig = entry.baseGeometry;
    double dw = orig.width()  * (scale - 1.0);
    double dh = orig.height() * (scale - 1.0);
    return QRect(
        static_cast<int>(orig.x() - dw / 2.0),
        static_cast<int>(orig.y() - dh / 2.0),
        static_cast<int>(orig.width()  * scale),
        static_cast<int>(orig.height() * scale)
    );
}

QJsonObject WidgetManager::setScale(const QJsonObject &params) {
    QString id = params.value("id").toString();
    double scale = params.value("scale").toDouble(1.0);
    int duration = params.value("duration").toInt(200);

    if (!m_widgets.contains(id)) {
        QJsonObject result;
        QJsonObject err;
        err["code"] = -32001;
        err["message"] = QString("Widget not found: %1").arg(id);
        result["error"] = err;
        return result;
    }

    WidgetEntry &entry = m_widgets[id];
    QRect targetRect = scaledGeometry(entry, scale);

    auto *anim = new QPropertyAnimation(entry.widget, "geometry");
    anim->setDuration(duration);
    anim->setStartValue(entry.widget->geometry());
    anim->setEndValue(targetRect);
    anim->setEasingCurve(QEasingCurve::OutCubic);
    anim->start(QAbstractAnimation::DeleteWhenStopped);

    entry.currentScale = scale;
    // baseGeometry 不更新——缩放始终以此为基准，保证 setScale(1.0) 能回到原位
    // KNOWN: mask 在动画开始时即更新为终态尺寸，动画中间帧的点击区域与视觉不一致。
    // Phase 3 正式方案：在 animationCompleted 事件回传后再更新 mask。
    updateClickMask();

    qDebug() << "[WidgetManager] setScale:" << id << "→" << scale
             << "duration" << duration << "ms";

    QJsonObject result;
    result["status"] = "ok";
    result["widgetId"] = id;
    return result;
}

QJsonObject WidgetManager::setOpacity(const QJsonObject &params) {
    QString id = params.value("id").toString();
    double opacity = params.value("opacity").toDouble(1.0);
    int duration = params.value("duration").toInt(150);

    if (!m_widgets.contains(id)) {
        QJsonObject result;
        QJsonObject err;
        err["code"] = -32001;
        err["message"] = QString("Widget not found: %1").arg(id);
        result["error"] = err;
        return result;
    }

    WidgetEntry &entry = m_widgets[id];
    QWidget *w = entry.widget;

    auto *effect = qobject_cast<QGraphicsOpacityEffect*>(w->graphicsEffect());
    if (!effect) {
        effect = new QGraphicsOpacityEffect(w);
        effect->setOpacity(entry.currentOpacity);
        w->setGraphicsEffect(effect);
    }

    auto *anim = new QPropertyAnimation(effect, "opacity");
    anim->setDuration(duration);
    anim->setStartValue(effect->opacity());
    anim->setEndValue(opacity);
    anim->setEasingCurve(QEasingCurve::OutCubic);
    anim->start(QAbstractAnimation::DeleteWhenStopped);

    entry.currentOpacity = opacity;

    qDebug() << "[WidgetManager] setOpacity:" << id << "→" << opacity;

    QJsonObject result;
    result["status"] = "ok";
    result["widgetId"] = id;
    return result;
}

void WidgetManager::updateClickMask() {
    QRegion region;
    for (auto it = m_widgets.begin(); it != m_widgets.end(); ++it) {
        QWidget *w = it.value().widget;
        if (w && w->isVisible()) {
            region += QRegion(w->geometry());
        }
    }
    // 只有已注册 Widget 时设 mask，空 region 会导致窗口不可见
    if (!region.isEmpty()) {
        m_container->setMask(region);
    }
}
