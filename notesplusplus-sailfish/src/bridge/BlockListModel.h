/* BlockListModel.h — High-performance QAbstractListModel for note blocks.
 *
 * Exposes individual block roles to QML delegates and emits targeted
 * dataChanged signals for in-place block updates and checkbox toggles,
 * avoiding complete view re-creation.
 */

#ifndef BLOCKLISTMODEL_H
#define BLOCKLISTMODEL_H

#include <QAbstractListModel>
#include <QJsonObject>
#include <QJsonArray>
#include <QJsonDocument>
#include <QVariantList>
#include <QVariantMap>
#include <QVector>

class BlockListModel : public QAbstractListModel
{
    Q_OBJECT
    Q_PROPERTY(int count READ count NOTIFY countChanged)

public:
    enum BlockRoles {
        BlockTypeRole = Qt::UserRole + 1,
        RawTextRole,
        RenderedHtmlRole,
        IsTaskRole,
        IsCheckedRole,
        IdRole,
        TitleRole,
        BlockDataRole
    };
    Q_ENUM(BlockRoles)

    explicit BlockListModel(QObject *parent = nullptr);
    ~BlockListModel() override = default;

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    QHash<int, QByteArray> roleNames() const override;

    Q_INVOKABLE int count() const;
    Q_INVOKABLE QVariantMap get(int index) const;
    Q_INVOKABLE void setBlocks(const QVariantList &blocks);
    Q_INVOKABLE void updateBlock(int index, const QVariantMap &data);
    Q_INVOKABLE void toggleCheckbox(int index, const QString &itemPath);
    Q_INVOKABLE void clear();
    Q_INVOKABLE QVariantList toVariantList() const;

signals:
    void countChanged();

private:
    QVector<QVariantMap> m_blocks;
};

#endif // BLOCKLISTMODEL_H
