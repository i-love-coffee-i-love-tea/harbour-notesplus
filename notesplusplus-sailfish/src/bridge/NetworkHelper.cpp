/* NetworkHelper.cpp — Network interface classification and preferred IP address resolution. */

#include "NetworkHelper.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSet>

bool NetworkHelper::isPublicIp(const QHostAddress &addr)
{
    if (addr.protocol() != QAbstractSocket::IPv4Protocol)
        return false;

    const quint32 ip = addr.toIPv4Address();
    const quint8 b0 = static_cast<quint8>((ip >> 24) & 0xFF);
    const quint8 b1 = static_cast<quint8>((ip >> 16) & 0xFF);
    const quint8 b2 = static_cast<quint8>((ip >> 8) & 0xFF);

    // Current network / Unspecified (0.0.0.0/8)
    if (b0 == 0) return false;
    // Loopback (127.0.0.0/8)
    if (b0 == 127) return false;
    // RFC 1918 (10.0.0.0/8)
    if (b0 == 10) return false;
    // RFC 1918 (172.16.0.0/12)
    if (b0 == 172 && (b1 >= 16 && b1 <= 31)) return false;
    // RFC 1918 (192.168.0.0/16)
    if (b0 == 192 && b1 == 168) return false;
    // Link-local / Cloud metadata (169.254.0.0/16)
    if (b0 == 169 && b1 == 254) return false;
    // Carrier-grade NAT (100.64.0.0/10)
    if (b0 == 100 && (b1 >= 64 && b1 <= 127)) return false;
    // IETF Protocol Assignments (192.0.0.0/24)
    if (b0 == 192 && b1 == 0 && b2 == 0) return false;
    // TEST-NET-1 (192.0.2.0/24)
    if (b0 == 192 && b1 == 0 && b2 == 2) return false;
    // Benchmarking (198.18.0.0/15)
    if (b0 == 198 && (b1 == 18 || b1 == 19)) return false;
    // TEST-NET-2 (198.51.100.0/24)
    if (b0 == 198 && b1 == 51 && b2 == 100) return false;
    // TEST-NET-3 (203.0.113.0/24)
    if (b0 == 203 && b1 == 0 && b2 == 113) return false;
    // Multicast (224.0.0.0/4) & Reserved (240.0.0.0/4) & Broadcast
    if (b0 >= 224) return false;
    if (ip == 0xFFFFFFFF) return false;

    return true;
}

bool NetworkHelper::isWifiInterface(const QString &name)
{
    const QString lower = name.toLower();
    return lower.startsWith(QLatin1String("wlan"))
        || lower.startsWith(QLatin1String("wl"))
        || lower.startsWith(QLatin1String("wifi"));
}

bool NetworkHelper::isBluetoothInterface(const QString &name)
{
    const QString lower = name.toLower();
    return lower.startsWith(QLatin1String("bnep"))
        || lower.startsWith(QLatin1String("bt"));
}

bool NetworkHelper::isWanInterface(const QString &name)
{
    const QString lower = name.toLower();
    return lower.startsWith(QLatin1String("ccmni"))
        || lower.startsWith(QLatin1String("rmnet"))
        || lower.startsWith(QLatin1String("wwan"))
        || lower.startsWith(QLatin1String("cellular"))
        || lower.startsWith(QLatin1String("ppp"))
        || lower.startsWith(QLatin1String("wan"));
}

QString NetworkHelper::resolvePreferredHostFromCandidates(bool rejectPublicNetworks,
                                                          const QString &wanPublicIp,
                                                          const QString &wifiIp,
                                                          const QString &btIp)
{
    // Priority 1: public if IP WAN enabled and IP available and public IPs not blocked
    if (!rejectPublicNetworks && !wanPublicIp.isEmpty()) {
        return wanPublicIp;
    }

    // Priority 2: WiFi IP, if available
    if (!wifiIp.isEmpty()) {
        return wifiIp;
    }

    // Priority 3: Bluetooth IP, if available
    if (!btIp.isEmpty()) {
        return btIp;
    }

    // Priority 4: localhost
    return QStringLiteral("localhost");
}

QString NetworkHelper::resolvePreferredHost(bool rejectPublicNetworks)
{
    QString wanPublicIp;
    QString wifiIp;
    QString btIp;

    const QList<QNetworkInterface> interfaces = QNetworkInterface::allInterfaces();
    for (const QNetworkInterface &iface : interfaces) {
        const QNetworkInterface::InterfaceFlags flags = iface.flags();
        if (!flags.testFlag(QNetworkInterface::IsUp))
            continue;
        if (flags.testFlag(QNetworkInterface::IsLoopBack))
            continue;

        const QString ifName = iface.name();
        const bool isWifi = isWifiInterface(ifName);
        const bool isBt   = isBluetoothInterface(ifName);
        const bool isWan  = isWanInterface(ifName);

        for (const QNetworkAddressEntry &entry : iface.addressEntries()) {
            const QHostAddress addr = entry.ip();
            if (addr.protocol() != QAbstractSocket::IPv4Protocol)
                continue;
            if (addr.isLoopback() || addr.isNull())
                continue;

            const QString ipStr = addr.toString();
            const bool isPublic = isPublicIp(addr);

            // Priority 1 candidate: public IP on IP WAN (or external interface)
            if (isPublic && (isWan || (!isWifi && !isBt)) && wanPublicIp.isEmpty()) {
                wanPublicIp = ipStr;
            }

            // Priority 2 candidate: WiFi IP
            if (isWifi && wifiIp.isEmpty()) {
                wifiIp = ipStr;
            }

            // Priority 3 candidate: Bluetooth IP
            if (isBt && btIp.isEmpty()) {
                btIp = ipStr;
            }
        }
    }

    return resolvePreferredHostFromCandidates(rejectPublicNetworks, wanPublicIp, wifiIp, btIp);
}

QString NetworkHelper::getNetworkInterfacesJson()
{
    QJsonArray array;
    QSet<QString> seenIps;

    // First entry: All interfaces
    QJsonObject allIfaces;
    allIfaces[QStringLiteral("ip")]   = QStringLiteral("0.0.0.0");
    allIfaces[QStringLiteral("name")] = QStringLiteral("All interfaces (0.0.0.0)");
    array.append(allIfaces);
    seenIps.insert(QStringLiteral("0.0.0.0"));

    const QList<QNetworkInterface> interfaces = QNetworkInterface::allInterfaces();
    for (const QNetworkInterface &iface : interfaces) {
        const QNetworkInterface::InterfaceFlags flags = iface.flags();
        if (!flags.testFlag(QNetworkInterface::IsUp))
            continue;
        if (flags.testFlag(QNetworkInterface::IsLoopBack))
            continue;

        const QString ifName = iface.name();
        QString typeLabel;
        if (isWifiInterface(ifName)) {
            typeLabel = QStringLiteral("WLAN");
        } else if (isBluetoothInterface(ifName)) {
            typeLabel = QStringLiteral("Bluetooth");
        } else if (isWanInterface(ifName)) {
            typeLabel = QStringLiteral("Cellular");
        } else {
            typeLabel = ifName;
        }

        for (const QNetworkAddressEntry &entry : iface.addressEntries()) {
            const QHostAddress addr = entry.ip();
            if (addr.protocol() != QAbstractSocket::IPv4Protocol)
                continue;
            if (addr.isLoopback() || addr.isNull())
                continue;

            const QString ipStr = addr.toString();
            if (!seenIps.contains(ipStr)) {
                seenIps.insert(ipStr);
                QJsonObject obj;
                obj[QStringLiteral("ip")]   = ipStr;
                obj[QStringLiteral("name")] = QStringLiteral("%1 (%2): %3").arg(typeLabel, ifName, ipStr);
                array.append(obj);
            }
        }
    }

    // Localhost only
    if (!seenIps.contains(QStringLiteral("127.0.0.1"))) {
        QJsonObject localObj;
        localObj[QStringLiteral("ip")]   = QStringLiteral("127.0.0.1");
        localObj[QStringLiteral("name")] = QStringLiteral("Localhost only (127.0.0.1)");
        array.append(localObj);
    }

    return QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact));
}
