#include "RenderServer.h"
#include "WidgetManager.h"

#include <QApplication>
#include <QTextStream>
#include <QJsonDocument>
#include <QJsonObject>
#include <QDebug>

// ---- StdinReader ----

StdinReader::StdinReader(QObject *parent) : QObject(parent) {}

void StdinReader::run() {
    QTextStream in(stdin);
    qDebug() << "[StdinReader] thread started, reading stdin...";

    while (true) {
        QString line = in.readLine();
        if (line.isNull()) {
            qDebug() << "[StdinReader] stdin closed";
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
        QJsonObject errResponse;
        errResponse["jsonrpc"] = QString("2.0");
        errResponse["id"] = QJsonValue(QJsonValue::Null);
        QJsonObject err;
        err["code"] = -32700;
        err["message"] = QString("Parse error");
        errResponse["error"] = err;
        sendResponse(errResponse);
        return;
    }

    QJsonObject response = handleMessage(doc.object());
    sendResponse(response);
}

QJsonObject RenderServer::handleMessage(const QJsonObject &msg) {
    QString jsonrpc = msg.value("jsonrpc").toString();
    if (jsonrpc != QString("2.0")) {
        QJsonObject err;
        err["jsonrpc"] = QString("2.0");
        err["id"] = msg.value("id");
        QJsonObject errObj;
        errObj["code"] = -32600;
        errObj["message"] = QString("Invalid Request: jsonrpc must be 2.0");
        err["error"] = errObj;
        return err;
    }

    QString method = msg.value("method").toString();
    if (method.isEmpty()) {
        QJsonObject err;
        err["jsonrpc"] = QString("2.0");
        err["id"] = msg.value("id");
        QJsonObject errObj;
        errObj["code"] = -32600;
        errObj["message"] = QString("Invalid Request: method is required");
        err["error"] = errObj;
        return err;
    }

    QJsonObject params = msg.value("params").toObject();
    QJsonValue id = msg.value("id");

    if (id.isUndefined()) {
        m_widgetManager->handleCommand(method, params);
        return QJsonObject();
    }

    QJsonObject result = m_widgetManager->handleCommand(method, params);

    QJsonObject response;
    response["jsonrpc"] = QString("2.0");
    response["id"] = id;

    if (result.contains("error")) {
        response["error"] = result["error"];
    } else {
        response["result"] = result;
    }

    return response;
}

void RenderServer::sendResponse(const QJsonObject &response) {
    if (response.isEmpty()) return;

    QTextStream out(stdout);
    QJsonDocument doc(response);
    out << doc.toJson(QJsonDocument::Compact) << "\n";
    out.flush();
}
