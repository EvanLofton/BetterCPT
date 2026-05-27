#ifndef RENDERSERVER_H
#define RENDERSERVER_H

#include <QObject>
#include <QThread>
#include <QJsonObject>

class WidgetManager;

// StdinReader — 在独立线程中阻塞读取 stdin，不阻塞 GUI 事件循环
class StdinReader : public QObject {
    Q_OBJECT
public:
    explicit StdinReader(QObject *parent = nullptr);

public slots:
    void run();  // 在线程中执行，阻塞读 stdin，逐行 emit lineRead

signals:
    void lineRead(const QString &line);
    void stdinClosed();
};

// RenderServer — 主线程，接收 StdinReader 发来的行并处理
class RenderServer : public QObject {
    Q_OBJECT
public:
    explicit RenderServer(WidgetManager *wm, QObject *parent = nullptr);
    ~RenderServer();

    void start();

private slots:
    void onLineRead(const QString &line);

private:
    QJsonObject handleMessage(const QJsonObject &msg);
    void sendResponse(const QJsonObject &response);

    WidgetManager *m_widgetManager;
    QThread *m_thread;
    StdinReader *m_reader;
};

#endif // RENDERSERVER_H
