#ifndef AGENTWIDGET_H
#define AGENTWIDGET_H

#include <QWidget>
#include "plugin_interface.h"

namespace Ui {
class AgentWidget;
}

class AgentWidget : public QWidget
{
    Q_OBJECT

public:
    explicit AgentWidget(QWidget *parent = nullptr);
    ~AgentWidget();

    // Method to update the UI with fresh metrics
    void updateMetrics(const QString& agentId, const system_metrics_t& metrics);

private:
    Ui::AgentWidget *ui;
};

#endif // AGENTWIDGET_H