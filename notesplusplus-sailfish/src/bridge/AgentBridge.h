/* AgentBridge.h — C++ QObject bridge exposing AI Assistant to Sailfish OS QML.
 *
 * This is the C++ port of the Rust agent_bridge.rs. QML sees IDENTICAL
 * property names, method names, and signal names.
 */

#ifndef AGENTBRIDGE_H
#define AGENTBRIDGE_H

#include <QObject>
#include <QString>
#include <QMutex>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QtConcurrent/QtConcurrent>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>

#include "../ffi/ffi_raii.h"

class AgentBridge : public QObject
{
    Q_OBJECT

    /* ---- Properties (21) ---- */
    Q_PROPERTY(bool    agent_busy          READ agentBusy         NOTIFY busy_changed)
    Q_PROPERTY(QString messages_json       READ messagesJson      NOTIFY messages_changed)
    Q_PROPERTY(QString pending_action_json READ pendingActionJson NOTIFY pending_action_changed)
    Q_PROPERTY(bool    has_pending_action  READ hasPendingAction  NOTIFY pending_action_changed)
    Q_PROPERTY(bool    can_undo            READ canUndo           NOTIFY undo_state_changed)
    Q_PROPERTY(QString last_snapshot_id    READ lastSnapshotId    NOTIFY undo_state_changed)
    Q_PROPERTY(QString last_created_note   READ lastCreatedNote   NOTIFY last_created_note_changed)
    Q_PROPERTY(QString error_message       READ errorMessage      NOTIFY error_occurred)
    Q_PROPERTY(QString streaming_text      READ streamingText     NOTIFY streaming_text_changed)
    Q_PROPERTY(QString step_status         READ stepStatus        NOTIFY step_status_changed)
    Q_PROPERTY(QString step_detail         READ stepDetail        NOTIFY step_status_changed)
    Q_PROPERTY(bool    is_fetching         READ isFetching        NOTIFY fetching_changed)

    // Configuration properties
    Q_PROPERTY(QString provider_type       READ providerType      NOTIFY config_changed)
    Q_PROPERTY(QString endpoint_url        READ endpointUrl       NOTIFY config_changed)
    Q_PROPERTY(QString model_name          READ modelName         NOTIFY config_changed)
    Q_PROPERTY(QString system_prompt       READ systemPrompt      NOTIFY config_changed)
    Q_PROPERTY(int     timeout_secs        READ timeoutSecs       NOTIFY config_changed)
    Q_PROPERTY(bool    auto_allow_read     READ autoAllowRead     NOTIFY config_changed)
    Q_PROPERTY(bool    auto_allow_create   READ autoAllowCreate   NOTIFY config_changed)
    Q_PROPERTY(bool    require_confirm_edit READ requireConfirmEdit NOTIFY config_changed)
    Q_PROPERTY(bool    allow_fetch_url     READ allowFetchUrl     NOTIFY config_changed)
    Q_PROPERTY(bool    allow_self_signed   READ allowSelfSigned   NOTIFY config_changed)
    Q_PROPERTY(QString available_models    READ availableModels   NOTIFY models_changed)
    Q_PROPERTY(bool    models_loading      READ modelsLoading     NOTIFY models_changed)

public:
    explicit AgentBridge(QObject *parent = nullptr);
    ~AgentBridge();

public slots:
    void appendStreamingToken(const QString &token);
    void handleStepStatus(const QString &status, const QString &detail);
    void handleSendResult(const QString &resultJson);
    void handleConfirmResult(const QString &resultJson);
    void handleUndoResult(const QString &resultJson);
    void handleFetchUrlResult(const QString &data, bool ok);
    void handleReadLocalFileResult(const QString &data, bool ok);

    /* ---- Property getters ---- */
    bool    agentBusy()         const { return m_agentBusy; }
    QString messagesJson()      const { return m_messagesJson; }
    QString pendingActionJson() const { return m_pendingActionJson; }
    bool    hasPendingAction()  const { return m_hasPendingAction; }
    bool    canUndo()           const { return m_canUndo; }
    QString lastSnapshotId()    const { return m_lastSnapshotId; }
    QString lastCreatedNote()   const { return m_lastCreatedNote; }
    QString errorMessage()      const { return m_errorMessage; }
    QString streamingText()     const { return m_streamingText; }
    QString stepStatus()        const { return m_stepStatus; }
    QString stepDetail()        const { return m_stepDetail; }
    bool    isFetching()        const { return m_isFetching; }
    QString providerType()      const { return m_providerType; }
    QString endpointUrl()       const { return m_endpointUrl; }
    QString modelName()         const { return m_modelName; }
    QString systemPrompt()      const { return m_systemPrompt; }
    int     timeoutSecs()       const { return m_timeoutSecs; }
    bool    autoAllowRead()     const { return m_autoAllowRead; }
    bool    autoAllowCreate()   const { return m_autoAllowCreate; }
    bool    requireConfirmEdit() const { return m_requireConfirmEdit; }
    bool    allowFetchUrl()     const { return m_allowFetchUrl; }
    bool    allowSelfSigned()   const { return m_allowSelfSigned; }
    QString availableModels()   const { return m_availableModels; }
    bool    modelsLoading()     const { return m_modelsLoading; }

    /* ---- Q_INVOKABLE methods (14) ---- */
    Q_INVOKABLE void configure(QString provider, QString url, QString model,
                               QString key, int timeout,
                               bool auto_read, bool auto_create,
                               bool require_edit, bool allow_self_signed,
                               bool allow_fetch,
                               QString system_prompt = QString());

    Q_INVOKABLE QString defaultSystemPrompt() const;

    Q_INVOKABLE void reset_session(QString context_filename,
                                   QString context_content,
                                   QString extra_context);

    Q_INVOKABLE void send_prompt(QString text);

    Q_INVOKABLE void run_template(QString template_id, QString input_text,
                                  QString context_filename,
                                  QString context_content);

    Q_INVOKABLE void run_custom_instruction(QString instruction,
                                            QString input_text,
                                            QString context_filename,
                                            QString context_content);

    Q_INVOKABLE void import_text(QString source_text, QString target_title,
                                 QString mode, QString custom_instruction);

    Q_INVOKABLE void fetch_url_content(QString url);

    Q_INVOKABLE void read_local_file(QString file_path);

    Q_INVOKABLE void confirm_action(bool approved);

    Q_INVOKABLE void undo_last_action();

    Q_INVOKABLE void cancel_operation();

    Q_INVOKABLE void fetch_models();

    Q_INVOKABLE bool poll_models();

signals:
    /* ---- Signals (14) ---- */
    void busy_changed();
    void messages_changed();
    void pending_action_changed();
    void undo_state_changed();
    void last_created_note_changed();
    void config_changed();
    void error_occurred(QString message);
    void response_finished(QString content);
    void undo_completed(QString message);
    void streaming_text_changed();
    void step_status_changed();
    void fetching_changed();
    void fetch_completed(QString result);
    void fetch_error(QString message);
    void models_changed();

private:
    /* ---- FFI session handle (RAII) ---- */
    AgentPtr m_session;

    /* ---- Internal config (not exposed to QML) ---- */
    QString m_internalApiKey;

    /* ---- Property storage ---- */
    bool    m_agentBusy         = false;
    QString m_messagesJson;
    QString m_pendingActionJson;
    bool    m_hasPendingAction  = false;
    bool    m_canUndo           = false;
    QString m_lastSnapshotId;
    QString m_lastCreatedNote;
    QString m_errorMessage;
    QString m_streamingText;
    QString m_stepStatus;
    QString m_stepDetail;
    bool    m_isFetching        = false;
    QString m_providerType;
    QString m_endpointUrl;
    QString m_modelName;
    QString m_systemPrompt;
    int     m_timeoutSecs       = 90;
    bool    m_autoAllowRead     = true;
    bool    m_autoAllowCreate   = true;
    bool    m_requireConfirmEdit = true;
    bool    m_allowFetchUrl     = true;
    bool    m_allowSelfSigned   = false;
    QString m_availableModels;
    bool    m_modelsLoading     = false;

    /* ---- Shared state for models worker ---- */
    QNetworkAccessManager *m_netManager = nullptr;

    /* ---- Helpers ---- */
    void reportError(const QString &msg);
    void beginOperation();
    void sendInBackground(const QString &prompt);
    void appendUserMessage(const QString &text);
    void appendAssistantMessage(const QString &text);
    void processAgentResult(const QString &resultJson);
    QString buildConfigJson() const;
    QString formatWithContext(const QString &instruction,
                              const QString &inputText,
                              const QString &contextFilename,
                              const QString &contextContent) const;
};

#endif // AGENTBRIDGE_H
