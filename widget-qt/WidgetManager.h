#ifndef WIDGETMANAGER_H
#define WIDGETMANAGER_H

#include <QObject>
#include <QWidget>
#include <QHash>
#include <QJsonObject>
#include <QRect>

struct WidgetEntry {
    QWidget *widget = nullptr;
    QRect baseGeometry;
    double currentScale = 1.0;
    double currentOpacity = 1.0;
    int radius = 0;
};

class WidgetManager : public QObject {
    Q_OBJECT
public:
    explicit WidgetManager(QObject *parent = nullptr);

    QJsonObject handleCommand(const QString &method, const QJsonObject &params);

private:
    // 创建
    QJsonObject createRect(const QJsonObject &p);
    QJsonObject createIcon(const QJsonObject &p);
    QJsonObject createText(const QJsonObject &p);
    // 动画
    QJsonObject setScale(const QJsonObject &p);
    QJsonObject setOpacity(const QJsonObject &p);
    // 层级
    QJsonObject addChild(const QJsonObject &p);
    QJsonObject removeChild(const QJsonObject &p);
    // Z-Order
    QJsonObject raiseWidget(const QJsonObject &p);
    QJsonObject lowerWidget(const QJsonObject &p);
    QJsonObject removeWidget(const QJsonObject &p);
    QJsonObject setWidgetColor(const QJsonObject &p);

    QRect scaledGeometry(const WidgetEntry &e, double scale) const;
    void updateContainerMask();
    QWidget* findWidget(const QString &id);

    QHash<QString, WidgetEntry> m_widgets;
};

#endif // WIDGETMANAGER_H
