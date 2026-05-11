#pragma once

#include <QMainWindow>
#include <QSystemTrayIcon>
#include <QCloseEvent>
#include <memory>
#include "natsclient.h"
#include "agentmodel.h"

QT_BEGIN_NAMESPACE
namespace Ui { class MainWindow; }
QT_END_NAMESPACE

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow();

protected:
    void closeEvent(QCloseEvent* event) override;

private slots:
    // Connection
    void onConnectClicked();
    void onDisconnectClicked();
    void onConnectionStatusChanged();
    
    // Agent actions
    void onAgentSelected();
    void onRefreshAgents();
    void onSendCommand();
    void onGetSystemInfo();
    void onGetProcesses();
    void onBrowseFiles();
    
    // NATS signals
    void onNatsConnected();
    void onNatsDisconnected();
    void onNatsError(const QString& error);
    void onAgentUpdated(const AgentInfo& agent);
    void onCommandResponse(const QString& agentId, const nlohmann::json& response);
    void onHeartbeatReceived(const QString& agentId, const nlohmann::json& metrics);
    
    // Tray icon
    void onTrayIconActivated(QSystemTrayIcon::ActivationReason reason);
    void onShowWindow();
    void onQuit();

private:
    void setupUi();
    void setupConnections();
    void setupTrayIcon();
    void updateAgentCount();
    void updateConnectionState(bool connected);
    void showNotification(const QString& title, const QString& message);
    void appendLog(const QString& message);

    Ui::MainWindow* ui;
    std::unique_ptr<NatsClient> m_natsClient;
    std::unique_ptr<AgentModel> m_agentModel;
    
    QSystemTrayIcon* m_trayIcon = nullptr;
    QAction* m_showAction = nullptr;
    QAction* m_quitAction = nullptr;
    
    QString m_currentAgentId;
    bool m_connected = false;
};
