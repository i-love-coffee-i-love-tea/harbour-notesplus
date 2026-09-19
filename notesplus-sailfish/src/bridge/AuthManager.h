/* AuthManager.h — Authentication challenge state.
 *
 * Manages the pending auth challenge lifecycle (check / approve / deny).
 * Polls the Rust core via FFI for challenges initiated by browser login.
 */

#ifndef AUTHMANAGER_H
#define AUTHMANAGER_H

#include <QString>
#include "../ffi/ffi_raii.h"

class AuthManager
{
public:
    /// Poll the Rust core for a pending auth challenge.
    /// Returns true if a challenge is (or was already) pending.
    bool check_auth_challenge(const HttpServerPtr &server)
    {
        if (!server) return m_authChallengePending;

        QString qjson = ffiStringToQString(notes_core_server_auth_take_challenge(server.get()));

        if (qjson.contains("\"pending\":true")) {
            int idStart = qjson.indexOf("\"challenge_id\":\"") + 16;
            int idEnd = qjson.indexOf("\"", idStart);
            int codeStart = qjson.indexOf("\"verification_code\":\"") + 21;
            int codeEnd = qjson.indexOf("\"", codeStart);

            if (idStart > 15 && idEnd > idStart)
                m_authChallengeId = qjson.mid(idStart, idEnd - idStart);
            if (codeStart > 20 && codeEnd > codeStart)
                m_authVerificationCode = qjson.mid(codeStart, codeEnd - codeStart);

            m_authChallengePending = true;
        }
        return m_authChallengePending;
    }

    void approve_auth_challenge(const QString &challenge_id, const HttpServerPtr &server)
    {
        if (server && !challenge_id.isEmpty()) {
            QByteArray utf8 = challenge_id.toUtf8();
            notes_core_server_auth_approve(server.get(), utf8.constData());
        }
        clear();
    }

    void deny_auth_challenge(const QString &challenge_id, const HttpServerPtr &server)
    {
        if (server && !challenge_id.isEmpty()) {
            QByteArray utf8 = challenge_id.toUtf8();
            notes_core_server_auth_deny(server.get(), utf8.constData());
        }
        clear();
    }

    bool    isPending()        const { return m_authChallengePending; }
    QString challengeId()      const { return m_authChallengeId; }
    QString verificationCode() const { return m_authVerificationCode; }

private:
    void clear()
    {
        m_authChallengePending = false;
        m_authChallengeId.clear();
        m_authVerificationCode.clear();
    }

    bool    m_authChallengePending = false;
    QString m_authChallengeId;
    QString m_authVerificationCode;
};

#endif /* AUTHMANAGER_H */
