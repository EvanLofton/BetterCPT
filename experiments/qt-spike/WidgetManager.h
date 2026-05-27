// ⚠️ Phase 0 Spike 代码，仅作历史参考。生产实现在 widget-qt/WidgetManager.h。
// 容器+子Widget 模型已废弃，Phase 3 起改为独立顶层窗口。
#ifndef WIDGETMANAGER_H
#define WIDGETMANAGER_H

#include <QObject>
#include <QWidget>
#include <QHash>
#include <QJsonObject>
#include <QJsonArray>
#include <QJsonValue>
#include <QRect>

struct WidgetEntry {
    QWidget *widget;
    QRect baseGeometry;  // 基准位置（scale=1.0 时的位置），仅创建/移动时更新
    double currentScale = 1.0;
    double currentOpacity = 1.0;
};

class WidgetManager : public QObject {
    Q_OBJECT
public:
    explicit WidgetManager(QWidget *container, QObject *parent = nullptr);

    QJsonObject handleCommand(const QString &method, const QJsonObject &params);

private:
    QJsonObject createRect(const QJsonObject &params);
    QJsonObject createIcon(const QJsonObject &params);
    QJsonObject createText(const QJsonObject &params);
    QJsonObject setScale(const QJsonObject &params);
    QJsonObject setOpacity(const QJsonObject &params);

    // 根据 scale 计算缩放后的 geometry（中心锚点）
    QRect scaledGeometry(const WidgetEntry &entry, double scale) const;

    // 重建容器 mask：只有已注册 Widget 的区域接收鼠标事件，其余穿透
    void updateClickMask();

    QWidget *m_container;
    QHash<QString, WidgetEntry> m_widgets;
};

#endif // WIDGETMANAGER_H
