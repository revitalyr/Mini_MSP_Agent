#include "agentwidget.h"
#include "ui_agentwidget.h"

AgentWidget::AgentWidget(QWidget *parent) :
    QWidget(parent),
    ui(new Ui::AgentWidget)
{
    ui->setupUi(this);
}

AgentWidget::~AgentWidget()
{
    delete ui;
}

void AgentWidget::updateMetrics(const QString& agentId, const system_metrics_t& metrics)
{
    QString details = QString(
        "Agent ID: %1\n"
        "Hostname: %2\n"
        "CPU Usage: %3%\n"
        "RAM Usage: %4%\n"
        "Disk Usage: %5%")
        .arg(agentId).arg(metrics.hostname).arg(metrics.cpu_usage, 0, 'f', 1).arg(metrics.ram_usage, 0, 'f', 1).arg(metrics.disk_usage, 0, 'f', 1);

    ui->label->setText(details);
}