#include "RenderServer.h"
#include "WidgetManager.h"

#include <QApplication>
#include <QTextStream>
#include <QJsonDocument>
#include <QJsonObject>
#include <QDebug>
#include <cstdio>

// ---- StdinReader ----
StdinReader::StdinReader(QObject *parent) : QObject(parent) {}

void StdinReader::run() {
    QTextStream in(stdin);
    qDebug() << "[RenderServer] IPC thread started, reading stdin...";

    while (true) {
        QString line = in.readLine();
        if (line.isNull()) {
            qDebug() << "[RenderServer] stdin closed";
            emit stdinClosed();
            break;
        }
        if (line.trimmed().isEmpty()) continue;
        emit lineRead(line);
    }
}

// ---- RenderServer ----
RenderServer::RenderServer(WidgetManager *wm, QObject *parent)
    : QObject(parent), m_widgetManager(wm)
{
    m_thread = new QThread(this);
    m_reader = new StdinReader();
    m_reader->moveToThread(m_thread);

    connect(m_thread, &QThread::started, m_reader, &StdinReader::run);
    connect(m_reader, &StdinReader::lineRead, this, &RenderServer::onLineRead);
    connect(m_reader, &StdinReader::stdinClosed, m_thread, &QThread::quit);
    connect(m_thread, &QThread::finished, QApplication::instance(), &QApplication::quit);
}

RenderServer::~RenderServer() {
    m_thread->quit();
    m_thread->wait(1000);
}

void RenderServer::start() {
    qDebug() << "[RenderServer] starting IPC thread...";
    m_thread->start();
}

void RenderServer::onLineRead(const QString &line) {
    QJsonParseError parseError;
    QJsonDocument doc = QJsonDocument::fromJson(line.toUtf8(), &parseError);

    if (parseError.error != QJsonParseError::NoError) {
        QByteArray err = "{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32700,\"message\":\"Parse error\"}}\n";
        fwrite(err.constData(), 1, err.size(), stdout); fflush(stdout);
        return;
    }

    QJsonObject response = handleMessage(doc.object());
    sendResponse(response);
}

QJsonObject RenderServer::handleMessage(const QJsonObject &msg) {
    QString jsonrpc = msg.value("jsonrpc").toString();
    if (jsonrpc != QString("2.0")) {
        QJsonObject err;
        err["jsonrpc"] = QString("2.0"); err["id"] = msg.value("id");
        err["error"] = QJsonObject{{"code", -32600}, {"message", "Invalid Request"}};
        return err;
    }

    QString method = msg.value("method").toString();
    if (method.isEmpty()) {
        QJsonObject err;
        err["jsonrpc"] = QString("2.0"); err["id"] = msg.value("id");
        err["error"] = QJsonObject{{"code", -32600}, {"message", "method required"}};
        return err;
    }

    QJsonObject params = msg.value("params").toObject();
    QJsonValue id = msg.value("id");

    if (id.isUndefined()) {
        m_widgetManager->handleCommand(method, params);
        return QJsonObject();
    }

    QJsonObject result = m_widgetManager->handleCommand(method, params);

    // String-based response construction (avoids QJsonObject issues)
    QByteArray resultJson = QJsonDocument(result).toJson(QJsonDocument::Compact);
    QByteArray resp;
    resp += "{\"jsonrpc\":\"2.0\",\"id\":";
    resp += QByteArray::number(id.toInt(0));
    if (result.contains("error")) {
        resp += ",\"error\":";
        resp += resultJson;
    } else {
        resp += ",\"result\":";
        resp += resultJson;
    }
    resp += "}";

    QJsonParseError pe;
    QJsonDocument respDoc = QJsonDocument::fromJson(resp, &pe);
    return pe.error == QJsonParseError::NoError ? respDoc.object() : QJsonObject();
}

void RenderServer::sendResponse(const QJsonObject &response) {
    if (response.isEmpty()) return;
    QByteArray json = QJsonDocument(response).toJson(QJsonDocument::Compact);
    json += '\n';
    fwrite(json.constData(), 1, json.size(), stdout);
    fflush(stdout);
}
