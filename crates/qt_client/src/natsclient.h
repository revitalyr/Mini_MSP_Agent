#pragma once

#include <QObject>
#include <QString>
#include <QByteArray>
#include <QTimer>
#include <QMap>
#include <QMutex>
#include <QDateTime>
#include <nats.h>
#include <nlohmann/json.hpp>

struct AgentInfo {
    QString id;
    QString hostname;
    QString platform;
    QString version;
    QDateTime lastSeen;
    double cpu = 0.0;
    double ram = 0.0;
    double disk = 0.0;
    int pluginCount = 0;
    bool online = false;
    
    nlohmann::json toJson() const;
    static AgentInfo fromJson(const nlohmann::json& j);
};

class NatsClient : public QObject
{
    Q_OBJECT

public:
    explicit NatsClient(QObject *parent = nullptr);
    ~NatsClient();

    bool connectToServer(const QString& url = "nats://localhost:4222");
    void disconnect();
    bool isConnected() const;

    // Commands
    bool sendCommand(const QString& agentId, const QString& command, 
                     const nlohmann::json& params = {});
    bool requestCommand(const QString& agentId, const QString& command,
                        const nlohmann::json& params = {},
                        int timeoutMs = 5000);

    // Subscriptions
    void subscribeToHeartbeats();
    void subscribeToAgentResponses(const QString& agentId);
    void subscribeToAllAgents();

    // Getters
    QMap<QString, AgentInfo> getAgents() const { return m_agents; }
    AgentInfo getAgent(const QString& id) const;

signals:
    void connected();
    void disconnected();
    void connectionError(const QString& error);
    void agentUpdated(const AgentInfo& agent);
    void agentRemoved(const QString& agentId);
    void commandResponse(const QString& agentId, const nlohmann::json& response);
    void heartbeatReceived(const QString& agentId, const nlohmann::json& metrics);

private slots:
    void processHeartbeatMessages();

private:
    static void onHeartbeatMsgCB(natsConnection* conn, natsSubscription* sub, 
                                  natsMsg* msg, void* closure);
    static void onResponseMsgCB(natsConnection* conn, natsSubscription* sub,
                                 natsMsg* msg, void* closure);
    static void onConnectionLostCB(natsConnection* conn, void* closure);
    static void onReconnectedCB(natsConnection* conn, void* closure);
    static void onClosedCB(natsConnection* conn, void* closure);
    static void onAsyncErrorCB(natsConnection* conn, natsSubscription* sub, natsStatus s, void* closure);

    void handleHeartbeat(const QByteArray& payload);
    void handleResponse(const QString& agentId, const QByteArray& payload);
    
    nlohmann::json decompressIfNeeded(const QByteArray& payload, 
                                       const char* encoding);

    natsConnection* m_conn = nullptr;
    natsSubscription* m_heartbeatSub = nullptr;
    QMap<QString, natsSubscription*> m_responseSubs;
    QMap<QString, AgentInfo> m_agents;
    QTimer* m_heartbeatTimer = nullptr;
    
    mutable QMutex m_agentsMutex;
};
