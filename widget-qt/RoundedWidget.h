#ifndef ROUNDEDWIDGET_H
#define ROUNDEDWIDGET_H

#include <QWidget>
#include <QColor>

class RoundedWidget : public QWidget {
    Q_OBJECT
public:
    explicit RoundedWidget(QWidget *parent = nullptr);

    void setBgColor(const QColor &color);
    void setRadius(int r);
    QColor bgColor() const { return m_bgColor; }
    int radius() const { return m_radius; }
    void enableFileDrop();

signals:
    void fileDropped(const QStringList &paths);

protected:
    void paintEvent(QPaintEvent *) override;
    bool nativeEvent(const QByteArray &eventType, void *message, qintptr *result) override;

private:
    QColor m_bgColor {30, 30, 30, 200};
    int m_radius = 0;
};

#endif
