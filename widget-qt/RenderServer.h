#ifndef RENDERSERVER_H
#define RENDERSERVER_H

#include <QObject>
#include <QThread>
#include <QJsonObject>

class WidgetManager;

class StdinReader : public QObject {
    Q_OBJECT
public:
    explicit StdinReader(QObject *parent = nullptr);
public slots:
    void run();
signals:
    void lineRead(const QString &line);
    void stdinClosed();
};

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

#endif
