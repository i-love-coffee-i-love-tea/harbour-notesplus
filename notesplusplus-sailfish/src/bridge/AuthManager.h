/* AuthManager.h — Authentication challenge state.
 *
 * Manages the pending auth challenge lifecycle (check / approve / deny).
 * Extracted from ServerManager so auth concerns are self-contained.
 */

#ifndef AUTHMANAGER_H
#define AUTHMANAGER_H

#include <QString>

class AuthManager
{
public:
    bool check_auth_challenge() const
    { return m_authChallengePending; }

    void approve_auth_challenge(const QString &challenge_id)
    {
        Q_UNUSED(challenge_id);
        m_authChallengePending = false;
        m_authChallengeId.clear();
        m_authVerificationCode.clear();
    }

    void deny_auth_challenge(const QString &challenge_id)
    {
        Q_UNUSED(challenge_id);
        m_authChallengePending = false;
        m_authChallengeId.clear();
        m_authVerificationCode.clear();
    }

    bool    isPending()        const { return m_authChallengePending; }
    QString challengeId()      const { return m_authChallengeId; }
    QString verificationCode() const { return m_authVerificationCode; }

private:
    bool    m_authChallengePending = false;
    QString m_authChallengeId;
    QString m_authVerificationCode;
};

#endif /* AUTHMANAGER_H */
