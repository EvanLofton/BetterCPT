#include "RoundedWidget.h"
#include <QPainter>
#include <QFileInfo>
#include <QDebug>
#include <Windows.h>
#include <shellapi.h>

RoundedWidget::RoundedWidget(QWidget *parent) : QWidget(parent) {
    setAttribute(Qt::WA_TranslucentBackground);
    setAutoFillBackground(false);
}

void RoundedWidget::setBgColor(const QColor &color) {
    m_bgColor = color;
    update();
}

void RoundedWidget::setRadius(int r) {
    m_radius = r;
    update();
}

void RoundedWidget::enableFileDrop() {
    if (!isWindow()) return; // 子 Widget 不注册拖放
    if (HWND hwnd = reinterpret_cast<HWND>(winId())) {
        DragAcceptFiles(hwnd, TRUE);
        qDebug() << "[RoundedWidget] DragAcceptFiles OK, hwnd=" << hwnd;
    }
}

bool RoundedWidget::nativeEvent(const QByteArray &, void *message, qintptr *result) {
    MSG *msg = static_cast<MSG *>(message);
    if (msg->message == WM_DROPFILES) {
        qDebug() << "[RoundedWidget] WM_DROPFILES received!";
        HDROP hDrop = reinterpret_cast<HDROP>(msg->wParam);
        UINT count = DragQueryFileW(hDrop, 0xFFFFFFFF, nullptr, 0);
        QStringList paths;
        for (UINT i = 0; i < count; i++) {
            wchar_t buf[MAX_PATH];
            DragQueryFileW(hDrop, i, buf, MAX_PATH);
            QString path = QString::fromWCharArray(buf);
            // .lnk 快捷方式 → 解析目标 exe
            if (path.endsWith(".lnk", Qt::CaseInsensitive)) {
                QFileInfo lnkInfo(path);
                paths.append(lnkInfo.absoluteFilePath()); // v1: 直接给 .lnk 路径，Phase 5 解析
            } else {
                paths.append(path);
            }
        }
        DragFinish(hDrop);
        emit fileDropped(paths);
        *result = 0;
        return true;
    }
    return false;
}

void RoundedWidget::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setCompositionMode(QPainter::CompositionMode_Source);
    painter.setBrush(Qt::transparent);
    painter.setPen(Qt::NoPen);
    painter.drawRect(rect());
    painter.setBrush(m_bgColor);
    if (m_radius > 0) {
        painter.drawRoundedRect(rect(), m_radius, m_radius);
    } else {
        painter.drawRect(rect());
    }
}
