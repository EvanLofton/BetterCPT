#include <QApplication>
#include <QWidget>
#include <QScreen>
#include <QRegion>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTextStream>
#include <QDebug>

#include "WidgetManager.h"
#include "RenderServer.h"

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);

    QScreen *screen = QApplication::primaryScreen();
    QRect screenGeo = screen->availableGeometry();

    // 创建 Overlay 容器窗口：无边框、置顶
    QWidget container;
    container.setWindowFlags(
        Qt::FramelessWindowHint
        | Qt::WindowStaysOnTopHint
    );
    container.setAttribute(Qt::WA_TranslucentBackground);

    int dockWidth = 600;
    int dockHeight = 80;
    container.setGeometry(
        (screenGeo.width() - dockWidth) / 2,
        screenGeo.height() - dockHeight - 10,
        dockWidth,
        dockHeight
    );

    // 容器初始全区域可见（mask = 整个容器）；图标创建后 mask 收窄到图标区域
    container.setMask(QRegion(0, 0, dockWidth, dockHeight));
    container.setStyleSheet("background: rgba(30, 30, 30, 200); border-radius: 16px;");

    container.show();

    // 初始化 WidgetManager 和 RenderServer
    WidgetManager widgetManager(&container);
    RenderServer renderServer(&widgetManager);

    // 发送 ready 通知到 stdout
    QJsonDocument readyDoc(QJsonObject{
        {"jsonrpc", "2.0"},
        {"method", "ready"},
        {"params", QJsonObject{}}
    });
    QTextStream out(stdout);
    out << readyDoc.toJson(QJsonDocument::Compact) << "\n";
    out.flush();

    qDebug() << "=== BetterCPT Qt Render Spike ===";
    qDebug() << "Screen:" << screenGeo;
    qDebug() << "Container window:" << container.geometry();
    qDebug() << "Waiting for IPC commands on stdin...";

    renderServer.start();
    return app.exec();
}
