#include "mainwindow.h"
#include "ui_mainwindow.h"
#include <QMessageBox>
#include <QInputDialog>
#include <QFileDialog>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QDateTime>
#include <QMenu>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , ui(new Ui::MainWindow)
    , m_natsClient(std::make_unique<NatsClient>())
    , m_agentModel(std::make_unique<AgentModel>())
{
    ui->setupUi(this);
    setupUi();
    setupConnections();
    setupTrayIcon();
    
    ui->agentTable->setModel(m_agentModel.get());
    ui->agentTable->horizontalHeader()->setStretchLastSection(true);
    ui->agentTable->horizontalHeader()->setSectionResizeMode(QHeaderView::ResizeToContents);
}

MainWindow::~MainWindow() = default;

void MainWindow::setupUi()
{
    setWindowTitle(tr("Mini MSP Qt Client"));
    resize(1200, 800);
    
    // Set default column widths
    ui->agentTable->setColumnWidth(0, 200); // ID
    ui->agentTable->setColumnWidth(1, 150); // Hostname
    ui->agentTable->setColumnWidth(2, 80);  // Platform
    ui->agentTable->setColumnWidth(3, 80);  // Status
    ui->agentTable->setColumnWidth(4, 60);  // CPU
    ui->agentTable->setColumnWidth(5, 60);  // RAM
    ui->agentTable->setColumnWidth(6, 60);  // Disk
    ui->agentTable->setColumnWidth(7, 60);  // Plugins
}

void MainWindow::setupConnections()
{
    // UI connections
    connect(ui->connectButton, &QPushButton::clicked, this, &MainWindow::onConnectClicked);
    connect(ui->disconnectButton, &QPushButton::clicked, this, &MainWindow::onDisconnectClicked);
    connect(ui->refreshButton, &QPushButton::clicked, this, &MainWindow::onRefreshAgents);
    connect(ui->agentTable->selectionModel(), &QItemSelectionModel::currentRowChanged,
            this, &MainWindow::onAgentSelected);
    
    // Command buttons
    connect(ui->systemInfoButton, &QPushButton::clicked, this, &MainWindow::onGetSystemInfo);
    connect(ui->processesButton, &QPushButton::clicked, this, &MainWindow::onGetProcesses);
    connect(ui->browseButton, &QPushButton::clicked, this, &MainWindow::onBrowseFiles);
    
    // Menu actions
    connect(ui->actionConnect, &QAction::triggered, this, &MainWindow::onConnectClicked);
    connect(ui->actionDisconnect, &QAction::triggered, this, &MainWindow::onDisconnectClicked);
    connect(ui->actionQuit, &QAction::triggered, this, &MainWindow::onQuit);
    connect(ui->actionRefresh, &QAction::triggered, this, &MainWindow::onRefreshAgents);
    connect(ui->actionClearLog, &QAction::triggered, ui->responseEdit, &QTextEdit::clear);
    
    // NATS client connections
    connect(m_natsClient.get(), &NatsClient::connected, this, &MainWindow::onNatsConnected);
    connect(m_natsClient.get(), &NatsClient::disconnected, this, &MainWindow::onNatsDisconnected);
    connect(m_natsClient.get(), &NatsClient::connectionError, this, &MainWindow::onNatsError);
    connect(m_natsClient.get(), &NatsClient::agentUpdated, this, &MainWindow::onAgentUpdated);
    connect(m_natsClient.get(), &NatsClient::commandResponse, this, &MainWindow::onCommandResponse);
    connect(m_natsClient.get(), &NatsClient::heartbeatReceived, this, &MainWindow::onHeartbeatReceived);
}

void MainWindow::setupTrayIcon()
{
    m_trayIcon = new QSystemTrayIcon(this);
    m_trayIcon->setIcon(QIcon::fromTheme("network-idle", QIcon(":/icons/app.png")));
    m_trayIcon->setToolTip(tr("Mini MSP Qt Client"));
    
    QMenu* trayMenu = new QMenu(this);
    m_showAction = trayMenu->addAction(tr("Show"), this, &MainWindow::onShowWindow);
    trayMenu->addSeparator();
    m_quitAction = trayMenu->addAction(tr("Quit"), this, &MainWindow::onQuit);
    
    m_trayIcon->setContextMenu(trayMenu);
    connect(m_trayIcon, &QSystemTrayIcon::activated, this, &MainWindow::onTrayIconActivated);
    
    m_trayIcon->show();
}

void MainWindow::closeEvent(QCloseEvent* event)
{
    if (m_trayIcon->isVisible()) {
        hide();
        event->ignore();
        showNotification(tr("Running in background"), 
                        tr("Mini MSP Qt Client is still running in the system tray."));
    } else {
        event->accept();
    }
}

// Connection slots
void MainWindow::onConnectClicked()
{
    QString url = ui->natsUrlEdit->text();
    if (url.isEmpty()) {
        QMessageBox::warning(this, tr("Error"), tr("Please enter NATS URL"));
        return;
    }
    
    appendLog(tr("Connecting to %1...").arg(url));
    
    if (!m_natsClient->connectToServer(url)) {
        QMessageBox::critical(this, tr("Connection Failed"), 
                             tr("Failed to connect to NATS server"));
        return;
    }
}

void MainWindow::onDisconnectClicked()
{
    m_natsClient->disconnect();
    appendLog(tr("Disconnected from NATS"));
}

void MainWindow::onConnectionStatusChanged()
{
    updateConnectionState(m_connected);
}

// Agent action slots
void MainWindow::onAgentSelected()
{
    auto index = ui->agentTable->currentIndex();
    if (!index.isValid()) return;
    
    AgentInfo agent = m_agentModel->getAgent(index);
    m_currentAgentId = agent.id;
    
    // Update info panel
    ui->idEdit->setText(agent.id);
    ui->hostnameEdit->setText(agent.hostname);
    ui->platformEdit->setText(agent.platform);
    
    // Enable command buttons
    bool hasAgent = !agent.id.isEmpty();
    ui->systemInfoButton->setEnabled(hasAgent);
    ui->processesButton->setEnabled(hasAgent);
    ui->browseButton->setEnabled(hasAgent);
}

void MainWindow::onRefreshAgents()
{
    // Force refresh by clearing and re-subscribing
    m_agentModel->clear();
    appendLog(tr("Agent list refreshed"));
}

void MainWindow::onSendCommand()
{
    if (m_currentAgentId.isEmpty()) {
        QMessageBox::warning(this, tr("Error"), tr("Please select an agent first"));
        return;
    }
    
    bool ok;
    QString command = QInputDialog::getText(this, tr("Send Command"),
                                           tr("Command:"), QLineEdit::Normal,
                                           tr("get_metrics"), &ok);
    if (!ok || command.isEmpty()) return;
    
    appendLog(tr("Sending command '%1' to agent %2...").arg(command, m_currentAgentId));
    
    nlohmann::json params = {};
    m_natsClient->sendCommand(m_currentAgentId, command, params);
}

void MainWindow::onGetSystemInfo()
{
    if (m_currentAgentId.isEmpty()) return;
    appendLog(tr("Requesting system info from %1...").arg(m_currentAgentId));
    m_natsClient->sendCommand(m_currentAgentId, "get_system_info");
}

void MainWindow::onGetProcesses()
{
    if (m_currentAgentId.isEmpty()) return;
    appendLog(tr("Requesting process list from %1...").arg(m_currentAgentId));
    m_natsClient->sendCommand(m_currentAgentId, "get_processes");
}

void MainWindow::onBrowseFiles()
{
    if (m_currentAgentId.isEmpty()) return;
    
    bool ok;
    QString path = QInputDialog::getText(this, tr("Browse Directory"),
                                        tr("Path:"), QLineEdit::Normal,
                                        tr("/home"), &ok);
    if (!ok) return;
    
    appendLog(tr("Browsing %1 on agent %2...").arg(path, m_currentAgentId));
    
    nlohmann::json params = {{"path", path.toStdString()}};
    m_natsClient->sendCommand(m_currentAgentId, "browse_directory", params);
}

// NATS signal handlers
void MainWindow::onNatsConnected()
{
    m_connected = true;
    updateConnectionState(true);
    appendLog(tr("Connected to NATS"));
    showNotification(tr("Connected"), tr("Successfully connected to NATS server"));
}

void MainWindow::onNatsDisconnected()
{
    m_connected = false;
    updateConnectionState(false);
    appendLog(tr("Disconnected from NATS"));
}

void MainWindow::onNatsError(const QString& error)
{
    appendLog(tr("Error: %1").arg(error));
    QMessageBox::critical(this, tr("NATS Error"), error);
}

void MainWindow::onAgentUpdated(const AgentInfo& agent)
{
    m_agentModel->updateAgent(agent);
    updateAgentCount();
    
    // Update info panel if this is the selected agent
    if (agent.id == m_currentAgentId) {
        ui->idEdit->setText(agent.id);
        ui->hostnameEdit->setText(agent.hostname);
        ui->platformEdit->setText(agent.platform);
    }
}

void MainWindow::onCommandResponse(const QString& agentId, const nlohmann::json& response)
{
    QString prettyResponse = QString::fromStdString(response.dump(2));
    appendLog(tr("Response from %1:\n%2").arg(agentId, prettyResponse));
}

void MainWindow::onHeartbeatReceived(const QString& agentId, const nlohmann::json& metrics)
{
    Q_UNUSED(agentId)
    Q_UNUSED(metrics)
    // Heartbeat received - agent is updated via onAgentUpdated
}

// Tray icon handlers
void MainWindow::onTrayIconActivated(QSystemTrayIcon::ActivationReason reason)
{
    if (reason == QSystemTrayIcon::DoubleClick) {
        onShowWindow();
    }
}

void MainWindow::onShowWindow()
{
    show();
    raise();
    activateWindow();
}

void MainWindow::onQuit()
{
    m_trayIcon->hide();
    qApp->quit();
}

// Helper methods
void MainWindow::updateAgentCount()
{
    int count = m_agentModel->rowCount();
    ui->agentCountLabel->setText(tr("Agents: %1").arg(count));
}

void MainWindow::updateConnectionState(bool connected)
{
    ui->connectButton->setEnabled(!connected);
    ui->disconnectButton->setEnabled(connected);
    ui->actionConnect->setEnabled(!connected);
    ui->actionDisconnect->setEnabled(connected);
    
    if (connected) {
        ui->statusLabel->setText(tr("Connected"));
        ui->statusLabel->setStyleSheet("color: green;");
        m_trayIcon->setIcon(QIcon::fromTheme("network-transmit-receive", 
                                             QIcon(":/icons/app-connected.png")));
    } else {
        ui->statusLabel->setText(tr("Disconnected"));
        ui->statusLabel->setStyleSheet("color: red;");
        m_trayIcon->setIcon(QIcon::fromTheme("network-offline", 
                                             QIcon(":/icons/app.png")));
    }
}

void MainWindow::showNotification(const QString& title, const QString& message)
{
    m_trayIcon->showMessage(title, message, QSystemTrayIcon::Information, 3000);
}

void MainWindow::appendLog(const QString& message)
{
    QString timestamp = QDateTime::currentDateTime().toString("yyyy-MM-dd hh:mm:ss");
    ui->responseEdit->append(QString("[%1] %2").arg(timestamp, message));
}
