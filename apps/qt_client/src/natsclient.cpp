#include "natsclient.h"
#include <QDebug>
#include <QMutexLocker>
#include <zstd.h>

using json = nlohmann::json;

// AgentInfo serialization
json AgentInfo::toJson() const {
    return json{
        {"id", id.toStdString()},
        {"hostname", hostname.toStdString()},
        {"platform", platform.toStdString()},
        {"version", version.toStdString()},
        {"last_seen", lastSeen.toSecsSinceEpoch()},
        {"cpu", cpu},
        {"ram", ram},
        {"disk", disk},
        {"plugin_count", pluginCount},
        {"online", online}
    };
}

AgentInfo AgentInfo::fromJson(const json& j) {
    AgentInfo info;
    info.id = QString::fromStdString(j.value("agent_id", j.value("id", "")));
    info.hostname = QString::fromStdString(j.value("hostname", ""));
    info.platform = QString::fromStdString(j.value("platform", ""));
    info.version = QString::fromStdString(j.value("version", "0.1.0"));
    info.lastSeen = QDateTime::fromSecsSinceEpoch(j.value("timestamp", 0));
    
    if (j.contains("metrics")) {
        auto& metrics = j["metrics"];
        info.cpu = metrics.value("cpu", 0.0);
        info.ram = metrics.value("ram", 0.0);
        info.disk = metrics.value("disk", 0.0);
    }
    
    info.pluginCount = j.value("plugin_count", 0);
    info.online = true;
    
    return info;
}

// NatsClient implementation
NatsClient::NatsClient(QObject *parent)
    : QObject(parent)
    , m_heartbeatTimer(new QTimer(this))
{
    m_heartbeatTimer->setInterval(100); // 100ms for processing messages
    connect(m_heartbeatTimer, &QTimer::timeout, this, &NatsClient::processHeartbeatMessages);
}

NatsClient::~NatsClient()
{
    disconnect();
}

bool NatsClient::connectToServer(const QString& url)
{
    natsStatus s = natsConnection_ConnectTo(&m_conn, url.toUtf8().constData());
    if (s != NATS_OK) {
        emit connectionError(QString("Failed to connect: %1").arg(natsStatus_GetText(s)));
        return false;
    }
    
    // Set connection lost handler
    natsConnection_SetDisconnectedCB(m_conn, onConnectionLostCB, this);
    
    emit connected();
    qDebug() << "Connected to NATS at" << url;
    
    // Start heartbeat processing
    m_heartbeatTimer->start();
    
    // Subscribe to heartbeats
    subscribeToHeartbeats();
    
    return true;
}

void NatsClient::disconnect()
{
    m_heartbeatTimer->stop();
    
    // Unsubscribe from all
    if (m_heartbeatSub) {
        natsSubscription_Unsubscribe(m_heartbeatSub);
        natsSubscription_Destroy(m_heartbeatSub);
        m_heartbeatSub = nullptr;
    }
    
    for (auto& sub : m_responseSubs) {
        natsSubscription_Unsubscribe(sub);
        natsSubscription_Destroy(sub);
    }
    m_responseSubs.clear();
    
    if (m_conn) {
        natsConnection_Destroy(m_conn);
        m_conn = nullptr;
    }
    
    emit disconnected();
}

bool NatsClient::isConnected() const
{
    return m_conn != nullptr && natsConnection_IsClosed(m_conn) == false;
}

bool NatsClient::sendCommand(const QString& agentId, const QString& command,
                              const nlohmann::json& params)
{
    if (!isConnected()) return false;
    
    json cmd = {
        {"command", command.toStdString()},
        {"params", params}
    };
    
    QString subject = QString("agent.%1.commands").arg(agentId);
    std::string payload = cmd.dump();
    
    natsStatus s = natsConnection_PublishString(m_conn, subject.toUtf8().constData(),
                                                 payload.c_str());
    if (s == NATS_OK) {
        s = natsConnection_Flush(m_conn);
    }
    
    return s == NATS_OK;
}

bool NatsClient::requestCommand(const QString& agentId, const QString& command,
                                 const nlohmann::json& params, int timeoutMs)
{
    if (!isConnected()) return false;
    
    json cmd = {
        {"command", command.toStdString()},
        {"params", params}
    };
    
    QString subject = QString("agent.%1.commands").arg(agentId);
    std::string payload = cmd.dump();
    
    natsMsg* reply = nullptr;
    natsStatus s = natsConnection_RequestString(&reply, m_conn, 
                                                  subject.toUtf8().constData(),
                                                  payload.c_str(), timeoutMs);
    
    if (s == NATS_OK && reply) {
        QByteArray data(natsMsg_GetData(reply), natsMsg_GetDataLength(reply));
        
        // Check for compression
        const char* encoding = natsMsg_GetHeaderValue(reply, "Content-Encoding");
        json response = decompressIfNeeded(data, encoding);
        
        emit commandResponse(agentId, response);
        natsMsg_Destroy(reply);
        return true;
    }
    
    return false;
}

void NatsClient::subscribeToHeartbeats()
{
    if (!isConnected()) return;
    
    natsStatus s = natsConnection_Subscribe(&m_heartbeatSub, m_conn, 
                                            "agent.heartbeat",
                                            onHeartbeatMsgCB, this);
    if (s != NATS_OK) {
        qDebug() << "Failed to subscribe to heartbeats:" << natsStatus_GetText(s);
    } else {
        qDebug() << "Subscribed to heartbeats";
    }
}

void NatsClient::subscribeToAgentResponses(const QString& agentId)
{
    if (!isConnected()) return;
    if (m_responseSubs.contains(agentId)) return;
    
    QString subject = QString("agent.%1.responses").arg(agentId);
    natsSubscription* sub = nullptr;
    
    natsStatus s = natsConnection_Subscribe(&sub, m_conn,
                                              subject.toUtf8().constData(),
                                              onResponseMsgCB, this);
    if (s == NATS_OK) {
        m_responseSubs[agentId] = sub;
        qDebug() << "Subscribed to responses for agent" << agentId;
    }
}

void NatsClient::subscribeToAllAgents()
{
    if (!isConnected()) return;
    
    // Subscribe to all agent responses
    natsSubscription* sub = nullptr;
    natsStatus s = natsConnection_Subscribe(&sub, m_conn,
                                              "agent.*.responses",
                                              onResponseMsgCB, this);
    if (s == NATS_OK) {
        qDebug() << "Subscribed to all agent responses";
    }
}

AgentInfo NatsClient::getAgent(const QString& id) const
{
    QMutexLocker locker(&m_agentsMutex);
    return m_agents.value(id);
}

void NatsClient::processHeartbeatMessages()
{
    // Process any pending messages (handled by callbacks)
    natsConnection_Flush(m_conn);
}

// Static callbacks
void NatsClient::onHeartbeatMsgCB(natsConnection* conn, natsSubscription* sub,
                                   natsMsg* msg, void* closure)
{
    Q_UNUSED(conn)
    Q_UNUSED(sub)
    
    auto* client = static_cast<NatsClient*>(closure);
    QByteArray payload(natsMsg_GetData(msg), natsMsg_GetDataLength(msg));
    
    client->handleHeartbeat(payload);
    natsMsg_Destroy(msg);
}

void NatsClient::onResponseMsgCB(natsConnection* conn, natsSubscription* sub,
                                  natsMsg* msg, void* closure)
{
    Q_UNUSED(conn)
    Q_UNUSED(sub)
    
    auto* client = static_cast<NatsClient*>(closure);
    
    // Extract agent ID from subject (agent.{id}.responses)
    QString subject = QString::fromUtf8(natsMsg_GetSubject(msg));
    QString agentId = subject.split('.').value(1, "");
    
    QByteArray payload(natsMsg_GetData(msg), natsMsg_GetDataLength(msg));
    
    const char* encoding = natsMsg_GetHeaderValue(msg, "Content-Encoding");
    client->handleResponse(agentId, client->decompressIfNeeded(payload, encoding));
    
    natsMsg_Destroy(msg);
}

void NatsClient::onConnectionLostCB(natsConnection* conn, void* closure)
{
    Q_UNUSED(conn)
    auto* client = static_cast<NatsClient*>(closure);
    emit client->connectionError("Connection to NATS lost");
}

void NatsClient::handleHeartbeat(const QByteArray& payload)
{
    try {
        json j = json::parse(payload.constData(), payload.constData() + payload.size());
        AgentInfo info = AgentInfo::fromJson(j);
        
        {
            QMutexLocker locker(&m_agentsMutex);
            bool isNew = !m_agents.contains(info.id);
            m_agents[info.id] = info;
        }
        
        emit agentUpdated(info);
        emit heartbeatReceived(info.id, j.value("metrics", json::object()));
        
        // Auto-subscribe to agent responses
        if (!m_responseSubs.contains(info.id)) {
            subscribeToAgentResponses(info.id);
        }
    } catch (const std::exception& e) {
        qDebug() << "Failed to parse heartbeat:" << e.what();
    }
}

void NatsClient::handleResponse(const QString& agentId, const QByteArray& payload)
{
    try {
        json j = json::parse(payload.constData(), payload.constData() + payload.size());
        emit commandResponse(agentId, j);
    } catch (const std::exception& e) {
        qDebug() << "Failed to parse response:" << e.what();
    }
}

json NatsClient::decompressIfNeeded(const QByteArray& payload, const char* encoding)
{
    if (encoding && QString::fromUtf8(encoding) == "zstd") {
        // Decompress with zstd
        size_t const dSize = ZSTD_decompressBound(payload.constData(), payload.size());
        if (dSize == ZSTD_CONTENTSIZE_ERROR) {
            qDebug() << "Could not determine decompressed size";
            return json::object();
        }
        
        std::vector<char> buffer(dSize);
        size_t const actualSize = ZSTD_decompress(buffer.data(), dSize,
                                                    payload.constData(), 
                                                    payload.size());
        if (ZSTD_isError(actualSize)) {
            qDebug() << "Decompression error:" << ZSTD_getErrorName(actualSize);
            return json::object();
        }
        
        return json::parse(buffer.data(), buffer.data() + actualSize);
    }
    
    return json::parse(payload.constData(), payload.constData() + payload.size());
}
