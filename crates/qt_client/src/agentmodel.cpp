#include "agentmodel.h"
#include <QBrush>
#include <QColor>

AgentModel::AgentModel(QObject *parent)
    : QAbstractTableModel(parent)
{
}

int AgentModel::rowCount(const QModelIndex& parent) const
{
    Q_UNUSED(parent)
    return m_agents.size();
}

int AgentModel::columnCount(const QModelIndex& parent) const
{
    Q_UNUSED(parent)
    return ColumnCount;
}

QVariant AgentModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid() || index.row() >= m_agents.size())
        return QVariant();

    const AgentInfo& agent = m_agents[index.row()];

    if (role == Qt::DisplayRole) {
        switch (index.column()) {
        case ColumnId:
            return agent.id;
        case ColumnHostname:
            return agent.hostname;
        case ColumnPlatform:
            return agent.platform;
        case ColumnStatus:
            return agent.online ? tr("Online") : tr("Offline");
        case ColumnCpu:
            return QString("%1%").arg(agent.cpu, 0, 'f', 1);
        case ColumnRam:
            return QString("%1%").arg(agent.ram, 0, 'f', 1);
        case ColumnDisk:
            return QString("%1%").arg(agent.disk, 0, 'f', 1);
        case ColumnPlugins:
            return agent.pluginCount;
        case ColumnLastSeen:
            return agent.lastSeen.toString("yyyy-MM-dd hh:mm:ss");
        default:
            return QVariant();
        }
    }
    else if (role == Qt::TextAlignmentRole) {
        if (index.column() >= ColumnCpu && index.column() <= ColumnPlugins) {
            return Qt::AlignCenter;
        }
        return Qt::AlignLeft;
    }
    else if (role == Qt::ForegroundRole) {
        if (index.column() == ColumnStatus) {
            return QBrush(agent.online ? QColor("#2ecc71") : QColor("#e74c3c"));
        }
        if (index.column() == ColumnCpu && agent.cpu > 80) {
            return QBrush(QColor("#e74c3c"));
        }
        if (index.column() == ColumnRam && agent.ram > 80) {
            return QBrush(QColor("#e74c3c"));
        }
    }
    else if (role == Qt::ToolTipRole) {
        return tr("Agent ID: %1\nHostname: %2\nPlatform: %3")
            .arg(agent.id, agent.hostname, agent.platform);
    }
    else if (role == Qt::UserRole) {
        // Return agent ID for retrieval
        return agent.id;
    }

    return QVariant();
}

QVariant AgentModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (role != Qt::DisplayRole || orientation != Qt::Horizontal)
        return QVariant();

    switch (section) {
    case ColumnId: return tr("Agent ID");
    case ColumnHostname: return tr("Hostname");
    case ColumnPlatform: return tr("Platform");
    case ColumnStatus: return tr("Status");
    case ColumnCpu: return tr("CPU %");
    case ColumnRam: return tr("RAM %");
    case ColumnDisk: return tr("Disk %");
    case ColumnPlugins: return tr("Plugins");
    case ColumnLastSeen: return tr("Last Seen");
    default: return QVariant();
    }
}

Qt::ItemFlags AgentModel::flags(const QModelIndex& index) const
{
    if (!index.isValid())
        return Qt::NoItemFlags;
    return Qt::ItemIsEnabled | Qt::ItemIsSelectable;
}

void AgentModel::updateAgent(const AgentInfo& agent)
{
    int row = findOrCreateRow(agent.id);
    
    if (row < m_agents.size()) {
        // Update existing
        m_agents[row] = agent;
        emit dataChanged(index(row, 0), index(row, ColumnCount - 1));
    } else {
        // Insert new
        beginInsertRows(QModelIndex(), row, row);
        m_agents.append(agent);
        endInsertRows();
    }
}

void AgentModel::removeAgent(const QString& agentId)
{
    auto it = m_idToRow.find(agentId);
    if (it == m_idToRow.end())
        return;

    int row = it.value();
    beginRemoveRows(QModelIndex(), row, row);
    m_agents.removeAt(row);
    
    // Rebuild row mapping
    m_idToRow.clear();
    for (int i = 0; i < m_agents.size(); ++i) {
        m_idToRow[m_agents[i].id] = i;
    }
    
    endRemoveRows();
}

void AgentModel::clear()
{
    beginResetModel();
    m_agents.clear();
    m_idToRow.clear();
    endResetModel();
}

AgentInfo AgentModel::getAgent(const QModelIndex& index) const
{
    if (!index.isValid() || index.row() >= m_agents.size())
        return AgentInfo();
    return m_agents[index.row()];
}

AgentInfo AgentModel::getAgent(const QString& id) const
{
    auto it = m_idToRow.find(id);
    if (it != m_idToRow.end()) {
        return m_agents[it.value()];
    }
    return AgentInfo();
}

QStringList AgentModel::getAgentIds() const
{
    return m_idToRow.keys();
}

int AgentModel::findOrCreateRow(const QString& agentId)
{
    auto it = m_idToRow.find(agentId);
    if (it != m_idToRow.end()) {
        return it.value();
    }
    
    int newRow = m_agents.size();
    m_idToRow[agentId] = newRow;
    return newRow;
}
