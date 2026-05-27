#include <QApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTextStream>
#include <QDebug>
#include <cstdio>
#include <Windows.h>
#include <combaseapi.h>

#include "WidgetManager.h"
#include "RenderServer.h"

int main(int argc, char *argv[]) {
    CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    QApplication app(argc, argv);

    WidgetManager widgetManager;
    RenderServer renderServer(&widgetManager);

    // 向 Host 发送 ready 通知（直接写入 stdout 避免 Qt 缓冲层）
    QByteArray readyJson = QJsonDocument(QJsonObject{
        {"jsonrpc", "2.0"},
        {"method", "ready"},
        {"params", QJsonObject{}}
    }).toJson(QJsonDocument::Compact);
    readyJson += '\n';
    fwrite(readyJson.constData(), 1, readyJson.size(), stdout);
    fflush(stdout);

    qDebug() << "=== BetterCPT Widget Qt ===";
    qDebug() << "Waiting for IPC commands on stdin...";

    renderServer.start();
    return app.exec();
}
