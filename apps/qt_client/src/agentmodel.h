#pragma once

#include <QAbstractTableModel>
#include <QList>
#include <QDateTime>
#include "natsclient.h"

class AgentModel : public QAbstractTableModel
{
    Q_OBJECT

public:
    enum Columns {
        ColumnId = 0,
        ColumnHostname,
        ColumnPlatform,
        ColumnStatus,
        ColumnCpu,
        ColumnRam,
        ColumnDisk,
        ColumnPlugins,
        ColumnLastSeen,
        ColumnCount
    };

    explicit AgentModel(QObject *parent = nullptr);

    // QAbstractTableModel interface
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    int columnCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role = Qt::DisplayRole) const override;
    QVariant headerData(int section, Qt::Orientation orientation, 
                        int role = Qt::DisplayRole) const override;
    Qt::ItemFlags flags(const QModelIndex& index) const override;

    void updateAgent(const AgentInfo& agent);
    void removeAgent(const QString& agentId);
    void clear();
    
    AgentInfo getAgent(const QModelIndex& index) const;
    AgentInfo getAgent(const QString& id) const;
    QStringList getAgentIds() const;

private:
    QList<AgentInfo> m_agents;
    QMap<QString, int> m_idToRow;
    
    int findOrCreateRow(const QString& agentId);
};
