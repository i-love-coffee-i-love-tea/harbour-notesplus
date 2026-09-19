/* NetworkHelper.h — Network interface classification and preferred IP address resolution.
 *
 * Implements address priority resolution for the embedded web server:
 * 1. Public IP if IP WAN is enabled, IP available, and public IPs are not blocked.
 * 2. WiFi IP, if available.
 * 3. Bluetooth IP, if available.
 * 4. Localhost ("localhost").
 */

#ifndef NETWORKHELPER_H
#define NETWORKHELPER_H

#include <QHostAddress>
#include <QList>
#include <QNetworkInterface>
#include <QString>

class NetworkHelper
{
public:
    /* Checks whether an address is a public routable IPv4 address.
     * Returns false for loopback, private (RFC 1918), CGNAT (RFC 6598),
     * link-local (RFC 3927), multicast, broadcast, and reserved ranges. */
    static bool isPublicIp(const QHostAddress &addr);

    /* Interface type classifiers based on Linux/Sailfish interface naming. */
    static bool isWifiInterface(const QString &name);
    static bool isBluetoothInterface(const QString &name);
    static bool isWanInterface(const QString &name);

    /* Resolves the preferred host address from given candidates according to priorities:
     * 1. wanPublicIp (if !rejectPublicNetworks and !wanPublicIp.isEmpty())
     * 2. wifiIp (if !wifiIp.isEmpty())
     * 3. btIp (if !btIp.isEmpty())
     * 4. "localhost" */
    static QString resolvePreferredHostFromCandidates(bool rejectPublicNetworks,
                                                      const QString &wanPublicIp,
                                                      const QString &wifiIp,
                                                      const QString &btIp);

    /* Queries active network interfaces and resolves the preferred host address. */
    static QString resolvePreferredHost(bool rejectPublicNetworks);

    /* Formats available network interfaces as a JSON array of objects:
     * [{"ip": "0.0.0.0", "name": "All interfaces (0.0.0.0)"}, ...] */
    static QString getNetworkInterfacesJson();
};

#endif /* NETWORKHELPER_H */
