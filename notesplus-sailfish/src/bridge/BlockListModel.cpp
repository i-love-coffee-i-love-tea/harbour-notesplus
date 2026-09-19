/* BlockListModel.cpp — Implementation of BlockListModel.
 */

#include "BlockListModel.h"

BlockListModel::BlockListModel(QObject *parent)
    : QAbstractListModel(parent)
{
}

int BlockListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid()) return 0;
    return m_blocks.size();
}

QVariant BlockListModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_blocks.size()) {
        return QVariant();
    }

    const QVariantMap &item = m_blocks.at(index.row());

    switch (role) {
    case BlockTypeRole:
        return item.value(QStringLiteral("block_type"), item.value(QStringLiteral("type")));
    case RawTextRole:
        return item.value(QStringLiteral("raw_text"), item.value(QStringLiteral("rawText")));
    case RenderedHtmlRole:
        return item.value(QStringLiteral("rendered_html"), item.value(QStringLiteral("renderedHtml")));
    case IsTaskRole:
        return item.value(QStringLiteral("is_task"), item.value(QStringLiteral("isTask")));
    case IsCheckedRole:
        return item.value(QStringLiteral("checked"), item.value(QStringLiteral("isChecked")));
    case IdRole:
        return item.value(QStringLiteral("id"));
    case TitleRole:
        return item.value(QStringLiteral("title"));
    case BlockDataRole:
    case Qt::DisplayRole:
        return item;
    default:
        return QVariant();
    }
}

QHash<int, QByteArray> BlockListModel::roleNames() const
{
    QHash<int, QByteArray> roles;
    roles[BlockTypeRole]    = "blockType";
    roles[RawTextRole]      = "rawText";
    roles[RenderedHtmlRole] = "renderedHtml";
    roles[IsTaskRole]       = "isTask";
    roles[IsCheckedRole]    = "isChecked";
    roles[IdRole]           = "id";
    roles[TitleRole]        = "title";
    roles[BlockDataRole]    = "blockData";
    return roles;
}

int BlockListModel::count() const
{
    return m_blocks.size();
}

QVariantMap BlockListModel::get(int index) const
{
    if (index >= 0 && index < m_blocks.size()) {
        return m_blocks.at(index);
    }
    return QVariantMap();
}

void BlockListModel::setBlocks(const QVariantList &blocks)
{
    beginResetModel();
    m_blocks.clear();
    m_blocks.reserve(blocks.size());

    for (const QVariant &v : blocks) {
        if (v.type() == QVariant::Map) {
            m_blocks.append(v.toMap());
        } else if (v.type() == QVariant::String) {
            QJsonDocument doc = QJsonDocument::fromJson(v.toString().toUtf8());
            if (doc.isObject()) {
                m_blocks.append(doc.object().toVariantMap());
            } else {
                m_blocks.append(QVariantMap());
            }
        }
    }

    endResetModel();
    emit countChanged();
}

void BlockListModel::updateBlock(int index, const QVariantMap &data)
{
    if (index < 0 || index >= m_blocks.size()) return;

    m_blocks[index] = data;
    QModelIndex modelIndex = createIndex(index, 0);
    emit dataChanged(modelIndex, modelIndex);
}

static QVariantMap toggleNestedCheckbox(QVariantMap node, const QStringList &parts, int depth)
{
    if (depth >= parts.size()) return node;
    bool ok = false;
    int childIdx = parts.at(depth).toInt(&ok);
    if (!ok) return node;

    if (depth == parts.size() - 1) {
        if (node.contains(QStringLiteral("blocks"))) {
            QVariantList subBlocks = node.value(QStringLiteral("blocks")).toList();
            if (childIdx >= 0 && childIdx < subBlocks.size()) {
                QVariantMap child = subBlocks.at(childIdx).toMap();
                if (child.contains(QStringLiteral("checked"))) {
                    child[QStringLiteral("checked")] = !child.value(QStringLiteral("checked")).toBool();
                    subBlocks[childIdx] = child;
                    node[QStringLiteral("blocks")] = subBlocks;
                }
            }
        }
        return node;
    }

    if (node.contains(QStringLiteral("blocks"))) {
        QVariantList subBlocks = node.value(QStringLiteral("blocks")).toList();
        if (childIdx >= 0 && childIdx < subBlocks.size()) {
            QVariantMap child = subBlocks.at(childIdx).toMap();
            subBlocks[childIdx] = toggleNestedCheckbox(child, parts, depth + 1);
            node[QStringLiteral("blocks")] = subBlocks;
        }
    }
    return node;
}

void BlockListModel::toggleCheckbox(int index, const QString &itemPath)
{
    if (index < 0 || index >= m_blocks.size()) return;

    QVariantMap block = m_blocks.at(index);

    if (itemPath.isEmpty()) {
        if (block.contains(QStringLiteral("checked"))) {
            block[QStringLiteral("checked")] = !block.value(QStringLiteral("checked")).toBool();
        }
    } else {
        QStringList parts = itemPath.split(QLatin1Char('.'));
        block = toggleNestedCheckbox(block, parts, 0);
    }

    m_blocks[index] = block;
    QModelIndex modelIndex = createIndex(index, 0);
    QVector<int> roles = {IsCheckedRole, BlockDataRole};
    emit dataChanged(modelIndex, modelIndex, roles);
}

void BlockListModel::clear()
{
    if (m_blocks.isEmpty()) return;
    beginResetModel();
    m_blocks.clear();
    endResetModel();
    emit countChanged();
}

QVariantList BlockListModel::toVariantList() const
{
    QVariantList list;
    list.reserve(m_blocks.size());
    for (const auto &item : m_blocks) {
        list.append(item);
    }
    return list;
}
