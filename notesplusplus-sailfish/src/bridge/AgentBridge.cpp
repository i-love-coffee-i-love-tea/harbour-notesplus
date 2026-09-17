/* AgentBridge.cpp — C++ QObject bridge exposing AI Assistant to Sailfish OS QML.
 *
 * C++ port of the Rust agent_bridge.rs. Uses the C FFI (notesplusplus_core.h)
 * with RAII wrappers (ffi_raii.h) for safe handle management.
 *
 * Threading model:
 *   - Blocking FFI calls (send, confirm, undo, fetch) run via QtConcurrent::run.
 *   - Streaming tokens arrive via FfiTokenCallback → queued to main thread.
 *   - Completion is signaled either by the streaming callback (isDone) or by
 *     the worker thread itself (fetch/undo/confirm), both via QTimer::singleShot.
 *   - Shared state between background threads and the main thread is protected
 *     by QMutex.
 */

#include "AgentBridge.h"

#include <QTimer>

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>

static constexpr int kDefaultAiTimeoutSecs = 90;

/* ================================================================== */
/* Constructor / Destructor                                           */
/* ================================================================== */

AgentBridge::AgentBridge(QObject *parent)
    : QObject(parent)
{
    /* ---- Resolve application paths via FFI ---- */
    AppPathsPtr paths(notes_core_app_paths_new());
    QString notesDir  = ffiStringToQString(notes_core_app_paths_notes_dir(paths.get()));
    QString dbPath    = ffiStringToQString(notes_core_app_paths_db_path(paths.get()));
    QString dataDir   = ffiStringToQString(notes_core_app_paths_data_dir(paths.get()));
    QString backupDir = dataDir + QStringLiteral("/backups");

    QDir().mkpath(notesDir);
    QDir().mkpath(backupDir);

    /* ---- Fetch default constants ---- */
    QString defaultEndpoint = ffiStringToQString(notes_core_const_default_ai_endpoint());
    QString defaultModel    = ffiStringToQString(notes_core_const_default_ai_model());

    /* ---- Set initial configuration properties ---- */
    m_providerType     = QStringLiteral("ollama");
    m_endpointUrl      = defaultEndpoint;
    m_modelName        = defaultModel;
    m_timeoutSecs      = kDefaultAiTimeoutSecs;
    m_autoAllowRead    = true;
    m_autoAllowCreate  = true;
    m_requireConfirmEdit = true;
    m_allowFetchUrl    = true;
    m_allowSelfSigned  = false;

    /* ---- Build initial config JSON ---- */
    QString configJson = buildConfigJson();

    /* ---- Create the FFI agent session ---- */
    m_session.reset(notes_core_agent_new(
        qstrToFFI(notesDir),
        qstrToFFI(dbPath),
        qstrToFFI(backupDir),
        qstrToFFI(configJson)));

    /* ---- Reset the conversation (no active note, no extra context) ---- */
    if (m_session) {
        notes_core_agent_reset_session(m_session.get(), nullptr, nullptr, nullptr);
    }

    /* ---- Initial messages are empty; populated on first interaction ---- */
    m_messagesJson = QStringLiteral("[]");
}

AgentBridge::~AgentBridge()
{
    // m_session (AgentPtr) is freed automatically by RAII unique_ptr
}

/* ================================================================== */
/* Private helpers                                                     */
/* ================================================================== */

void AgentBridge::reportError(const QString &msg)
{
    m_errorMessage = msg;
    emit error_occurred(m_errorMessage);
}

void AgentBridge::beginOperation()
{
    m_agentBusy = true;
    m_streamingText.clear();
    m_streamingActivity = false;
    m_completionIdleRetries = 0;
    emit busy_changed();
    emit streaming_text_changed();
}

/* Append a user message to the local messages_json for immediate UI feedback.
 * Mirrors Rust: session.push_user_message(&text) */
void AgentBridge::appendUserMessage(const QString &text)
{
    QJsonDocument doc = QJsonDocument::fromJson(m_messagesJson.toUtf8());
    QJsonArray arr = doc.isArray() ? doc.array() : QJsonArray();

    QJsonObject userMsg;
    userMsg[QStringLiteral("role")]    = QStringLiteral("user");
    userMsg[QStringLiteral("content")] = text;
    arr.append(userMsg);

    m_messagesJson = QString::fromUtf8(
        QJsonDocument(arr).toJson(QJsonDocument::Compact));
    emit messages_changed();
}

/* ================================================================== */
/* Streaming callback (called from Rust worker thread)                 */
/* ================================================================== */

static void agentStreamingTokenCallback(void *userData, const char *token, int isDone)
{
    AgentBridge *bridge = static_cast<AgentBridge*>(userData);
    if (!bridge) return;

    if (token) {
        QString tokenStr = QString::fromUtf8(token);
        QTimer::singleShot(0, bridge, [bridge, tokenStr]() {
            bridge->appendStreamingToken(tokenStr);
        });
    }
    if (isDone) {
        QTimer::singleShot(0, bridge, [bridge]() {
            bridge->handleAgentCompletion();
        });
    }
}

void AgentBridge::appendStreamingToken(const QString &token)
{
    m_streamingText += token;
    m_streamingActivity = true;
    emit streaming_text_changed();
}

/* Called (via QTimer::singleShot) when the streaming callback reports
 * isDone. Polls the Rust side for the final result.  In multi-step
 * operations (tool calls), isDone fires after each LLM turn, but the
 * agent continues executing.  We keep polling as long as streaming
 * tokens keep arriving (m_streamingActivity resets the deadline).
 * Once tokens stop, we wait up to 2 s for the final result. */
void AgentBridge::handleAgentCompletion()
{
    if (!m_session) return;

    char *outJson = nullptr;
    int status = notes_core_agent_poll(m_session.get(), &outJson);

    if (status == 0) {
        /* Still running — check if tokens are still arriving. */
        if (m_streamingActivity) {
            /* New tokens arrived since the last check — the agent is
             * actively working (e.g., executing tools).  Reset and
             * retry soon. */
            m_streamingActivity = false;
            m_completionIdleRetries = 0;
            QTimer::singleShot(50, this, [this]() {
                handleAgentCompletion();
            });
        } else {
            /* No new tokens — give the agent up to 2 s to finish. */
            if (m_completionIdleRetries < 40) {   // 40 × 50 ms = 2 s
                ++m_completionIdleRetries;
                QTimer::singleShot(50, this, [this]() {
                    handleAgentCompletion();
                });
            } else {
                m_completionIdleRetries = 0;
                /* Timed out — force completion. */
                m_agentBusy = false;
                m_streamingText.clear();
                emit streaming_text_changed();
                emit busy_changed();
            }
        }
        return;
    }

    /* status == 1 (success) or status == -1 (error) — done. */
    m_completionIdleRetries = 0;

    QString resultJson;
    if (status == 1 && outJson) {
        resultJson = ffiStringToQString(outJson);
    }

    if (!resultJson.isEmpty()) {
        processAgentResult(resultJson);
    }

    m_agentBusy = false;
    m_streamingText.clear();
    emit streaming_text_changed();
    emit busy_changed();

    if (status == -1) {
        reportError(QStringLiteral("Agent operation failed"));
    }
}

/* ================================================================== */
/* Start an agent send (streaming) — tokens arrive via callback.       */
/* ================================================================== */

void AgentBridge::sendInBackground(const QString &prompt)
{
    beginOperation();

    QtConcurrent::run([this, prompt]() {
        notes_core_agent_send_streaming(
            m_session.get(),
            qstrToFFI(prompt),
            &agentStreamingTokenCallback,
            this);
    });
}

/* Format a prompt with optional context (active note content).
 * Used by run_template, run_custom_instruction, import_text. */
QString AgentBridge::formatWithContext(const QString &instruction,
                                       const QString &inputText,
                                       const QString &contextFilename,
                                       const QString &contextContent) const
{
    QString prompt;
    prompt += instruction;
    prompt += QStringLiteral("\n\n");

    if (!contextFilename.trimmed().isEmpty()) {
        prompt += QStringLiteral("### Active Note: ") + contextFilename + QStringLiteral("\n");
        if (!contextContent.trimmed().isEmpty()) {
            prompt += QStringLiteral("```\n") + contextContent + QStringLiteral("\n```\n");
        }
        prompt += QStringLiteral("\n");
    }

    if (!inputText.trimmed().isEmpty()) {
        prompt += QStringLiteral("### Input:\n") + inputText;
    }

    return prompt.trimmed();
}

/* Build the JSON configuration blob consumed by notes_core_agent_new / _configure.
 * Must match the format expected by the Rust side. */
QString AgentBridge::buildConfigJson() const
{
    QJsonObject cfg;
    cfg[QStringLiteral("provider")]          = m_providerType;
    cfg[QStringLiteral("endpoint_url")]      = m_endpointUrl;
    cfg[QStringLiteral("model")]             = m_modelName;
    if (!m_internalApiKey.trimmed().isEmpty()) {
        cfg[QStringLiteral("api_key")]       = m_internalApiKey;
    } else {
        cfg[QStringLiteral("api_key")]       = QJsonValue::Null;
    }
    cfg[QStringLiteral("timeout_secs")]      = m_timeoutSecs;
    cfg[QStringLiteral("allow_self_signed")] = m_allowSelfSigned;
    cfg[QStringLiteral("auto_allow_read")]   = m_autoAllowRead;
    cfg[QStringLiteral("auto_allow_create")] = m_autoAllowCreate;
    cfg[QStringLiteral("require_confirm_edit")] = m_requireConfirmEdit;
    cfg[QStringLiteral("allow_fetch_url")]   = m_allowFetchUrl;
    return QString::fromUtf8(QJsonDocument(cfg).toJson(QJsonDocument::Compact));
}

/* Process the JSON result returned by notes_core_agent_poll().
 *
 * Expected format (mirrors Rust WorkerOutput serialization):
 * {
 *   "step_result": { "type": "Finished"|"Error"|"RequiresConfirmation",
 *                    "content": "..." },
 *   "messages_json": "[ ... ]",
 *   "pending_action_json": "{ ... }" | null,
 *   "can_undo": true|false,
 *   "last_snapshot_id": "..." | null,
 *   "last_created_note": "..." | null
 * }
 */
void AgentBridge::processAgentResult(const QString &resultJson)
{
    QJsonDocument doc = QJsonDocument::fromJson(resultJson.toUtf8());
    if (!doc.isObject()) {
        qWarning("[AgentBridge] poll returned non-JSON: %s",
                 resultJson.left(200).toUtf8().constData());
        return;
    }
    QJsonObject obj = doc.object();

    /* ---- messages ---- */
    if (obj.contains(QStringLiteral("messages_json"))) {
        m_messagesJson = obj[QStringLiteral("messages_json")].toString();
        emit messages_changed();
    }

    /* ---- pending action ---- */
    QJsonValue paVal = obj[QStringLiteral("pending_action_json")];
    if (!paVal.isNull() && paVal.isString() && !paVal.toString().isEmpty()) {
        m_pendingActionJson = paVal.toString();
        m_hasPendingAction  = true;
    } else {
        m_pendingActionJson.clear();
        m_hasPendingAction = false;
    }
    emit pending_action_changed();

    /* ---- undo / snapshot state ---- */
    m_canUndo        = obj[QStringLiteral("can_undo")].toBool(false);
    m_lastSnapshotId = obj[QStringLiteral("last_snapshot_id")].toString();
    emit undo_state_changed();

    m_lastCreatedNote = obj[QStringLiteral("last_created_note")].toString();
    emit last_created_note_changed();

    /* ---- step result ---- */
    QJsonObject step = obj[QStringLiteral("step_result")].toObject();
    QString type = step[QStringLiteral("type")].toString();

    if (type == QStringLiteral("Finished")) {
        QString content = step[QStringLiteral("content")].toString();
        emit response_finished(content);
        if (content.startsWith(QStringLiteral("Successfully rolled back"))) {
            emit undo_completed(content);
        }
    } else if (type == QStringLiteral("Error")) {
        QString errMsg = step[QStringLiteral("content")].toString();
        if (errMsg.isEmpty())
            errMsg = step[QStringLiteral("error")].toString();
        reportError(errMsg);
    }
    // type == "RequiresConfirmation" → pending_action already updated above
}

/* ================================================================== */
/* One-shot completion handler for fetch/undo/confirm workers.         */
/* Called once via QTimer::singleShot from the worker thread.          */
/* ================================================================== */

void AgentBridge::handleWorkerCompletion()
{
    /* ---- 1. Check fetch-worker result (URL / file) ---- */
    {
        QMutexLocker lock(&m_fetchMutex);
        if (m_fetchReady) {
            m_fetchReady = false;
            bool ok       = m_fetchSuccess;
            QString data  = m_fetchContent;
            lock.unlock();

            m_isFetching = false;
            emit fetching_changed();
            if (ok) {
                emit fetch_completed(data);
            } else {
                emit fetch_error(data);
            }
        }
    }

    /* ---- 2. Check undo-worker result ---- */
    {
        QMutexLocker lock(&m_undoMutex);
        if (m_undoReady) {
            m_undoReady = false;
            bool ok        = m_undoSuccess;
            QString msg    = m_undoMessage;
            lock.unlock();

            if (ok) {
                // Get updated session state
                char *pollJson = nullptr;
                int ps = notes_core_agent_poll(m_session.get(), &pollJson);
                if (ps == 1 && pollJson) {
                    processAgentResult(ffiStringToQString(pollJson));
                }
                emit undo_completed(msg);
            } else {
                reportError(msg);
            }
            m_agentBusy = false;
            emit busy_changed();
        }
    }
}

/* ================================================================== */
/* Q_INVOKABLE: configure                                             */
/* ================================================================== */

void AgentBridge::configure(QString provider, QString url, QString model,
                            QString key, int timeout,
                            bool auto_read, bool auto_create,
                            bool require_edit, bool allow_self_signed,
                            bool allow_fetch)
{
    m_providerType       = provider;
    m_endpointUrl        = url;
    m_modelName          = model;
    m_internalApiKey     = key;
    m_timeoutSecs        = (timeout > 0) ? timeout : kDefaultAiTimeoutSecs;
    m_autoAllowRead      = auto_read;
    m_autoAllowCreate    = auto_create;
    m_requireConfirmEdit = require_edit;
    m_allowSelfSigned    = allow_self_signed;
    m_allowFetchUrl      = allow_fetch;

    if (m_session) {
        notes_core_agent_configure(m_session.get(), qstrToFFI(buildConfigJson()));
    }

    emit config_changed();
}

/* ================================================================== */
/* Q_INVOKABLE: reset_session                                          */
/* ================================================================== */

void AgentBridge::reset_session(QString context_filename,
                                QString context_content,
                                QString extra_context)
{
    if (!m_session) return;

    const char *fn    = context_filename.trimmed().isEmpty()
                            ? nullptr : qstrToFFI(context_filename);
    const char *fc    = context_content.trimmed().isEmpty()
                            ? nullptr : qstrToFFI(context_content);
    const char *extra = extra_context.trimmed().isEmpty()
                            ? nullptr : qstrToFFI(extra_context);

    notes_core_agent_reset_session(m_session.get(), fn, fc, extra);

    // After reset, messages are back to initial state
    m_messagesJson      = QStringLiteral("[]");
    m_pendingActionJson.clear();
    m_hasPendingAction  = false;

    emit messages_changed();
    emit pending_action_changed();
}

/* ================================================================== */
/* Q_INVOKABLE: send_prompt                                            */
/* ================================================================== */

void AgentBridge::send_prompt(QString text)
{
    if (m_agentBusy || text.trimmed().isEmpty()) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    // Show user message in UI immediately (mirrors Rust push_user_message)
    appendUserMessage(text);

    sendInBackground(text);
}

/* ================================================================== */
/* Q_INVOKABLE: run_template                                           */
/* ================================================================== */

void AgentBridge::run_template(QString template_id, QString input_text,
                               QString context_filename,
                               QString context_content)
{
    if (m_agentBusy) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    QString instruction = QStringLiteral("[Apply template \"")
                          + template_id + QStringLiteral("\"]");
    QString prompt = formatWithContext(instruction, input_text,
                                      context_filename, context_content);
    if (prompt.isEmpty()) return;

    appendUserMessage(prompt);
    sendInBackground(prompt);
}

/* ================================================================== */
/* Q_INVOKABLE: run_custom_instruction                                  */
/* ================================================================== */

void AgentBridge::run_custom_instruction(QString instruction,
                                         QString input_text,
                                         QString context_filename,
                                         QString context_content)
{
    if (m_agentBusy || instruction.trimmed().isEmpty()) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    QString instHeader = QStringLiteral("[Custom instruction: ")
                         + instruction + QStringLiteral("]");
    QString prompt = formatWithContext(instHeader, input_text,
                                      context_filename, context_content);
    if (prompt.isEmpty()) return;

    appendUserMessage(prompt);
    sendInBackground(prompt);
}

/* ================================================================== */
/* Q_INVOKABLE: import_text                                            */
/* ================================================================== */

void AgentBridge::import_text(QString source_text, QString target_title,
                              QString mode, QString custom_instruction)
{
    if (m_agentBusy || source_text.trimmed().isEmpty()) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    QString header = QStringLiteral("[Import text — mode: ") + mode + QStringLiteral("]");
    if (!target_title.trimmed().isEmpty()) {
        header += QStringLiteral("\nTarget title: ") + target_title;
    }
    if (!custom_instruction.trimmed().isEmpty()) {
        header += QStringLiteral("\nCustom instruction: ") + custom_instruction;
    }

    QString prompt = header + QStringLiteral("\n\nSource text:\n") + source_text;

    appendUserMessage(prompt);
    sendInBackground(prompt);
}

/* ================================================================== */
/* Q_INVOKABLE: fetch_url_content                                      */
/* ================================================================== */

void AgentBridge::fetch_url_content(QString url)
{
    if (m_isFetching) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    m_isFetching = true;
    emit fetching_changed();

    {
        QMutexLocker lock(&m_fetchMutex);
        m_fetchReady = false;
    }

    QtConcurrent::run([this, url]() {
        char *raw = notes_core_fetch_url(qstrToFFI(url));

        QMutexLocker lock(&m_fetchMutex);
        if (raw) {
            m_fetchContent = ffiStringToQString(raw);
            m_fetchSuccess = true;
        } else {
            m_fetchContent = QStringLiteral("Error: fetch returned null");
            m_fetchSuccess = false;
        }
        m_fetchReady = true;

        QTimer::singleShot(0, this, [this]() {
            this->handleWorkerCompletion();
        });
    });
}

/* ================================================================== */
/* Q_INVOKABLE: read_local_file                                        */
/* ================================================================== */

void AgentBridge::read_local_file(QString file_path)
{
    QString p = file_path.trimmed();
    if (p.isEmpty()) return;

    // Path-traversal guard (mirrors Rust)
    if (p.contains(QStringLiteral(".."))) {
        emit fetch_error(QStringLiteral("Error: path traversal ('..') is not allowed"));
        return;
    }

    if (m_isFetching) return;

    m_isFetching = true;
    emit fetching_changed();

    {
        QMutexLocker lock(&m_fetchMutex);
        m_fetchReady = false;
    }

    QtConcurrent::run([this, p]() {
        /* Resolve ~ to home directory */
        QString expanded = p;
        if (expanded.startsWith(QStringLiteral("~/"))) {
            expanded = QDir::homePath() + expanded.mid(1);
        }

        /* Obtain notes_dir for containment check */
        AppPathsPtr paths(notes_core_app_paths_new());
        QString notesDir = ffiStringToQString(
            notes_core_app_paths_notes_dir(paths.get()));

        /* Canonicalize */
        QFileInfo fi(expanded);
        QString canonical = fi.canonicalFilePath();
        if (canonical.isEmpty()) {
            QMutexLocker lock(&m_fetchMutex);
            m_fetchContent = QStringLiteral("Error: could not resolve path: ") + expanded;
            m_fetchSuccess = false;
            m_fetchReady   = true;
            return;
        }

        /* Containment check — must be inside notes_dir */
        if (!canonical.startsWith(notesDir)) {
            QMutexLocker lock(&m_fetchMutex);
            m_fetchContent = QStringLiteral(
                "Error: access denied — file is outside the notes directory");
            m_fetchSuccess = false;
            m_fetchReady   = true;
            return;
        }

        /* Read the file */
        QFile file(canonical);
        if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QMutexLocker lock(&m_fetchMutex);
            m_fetchContent = QStringLiteral("Error reading file: ")
                             + file.errorString();
            m_fetchSuccess = false;
            m_fetchReady   = true;
            return;
        }

        QString content = QString::fromUtf8(file.readAll());

        QMutexLocker lock(&m_fetchMutex);
        m_fetchContent = content;
        m_fetchSuccess = true;
        m_fetchReady   = true;

        QTimer::singleShot(0, this, [this]() {
            this->handleWorkerCompletion();
        });
    });
}

/* ================================================================== */
/* Q_INVOKABLE: confirm_action                                         */
/* ================================================================== */

void AgentBridge::confirm_action(bool approved)
{
    if (m_agentBusy || !m_hasPendingAction) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    beginOperation();

    QtConcurrent::run([this, approved]() {
        notes_core_agent_confirm(m_session.get(), approved ? 1 : 0);

        /* Fetch final result immediately — confirm is a one-shot operation,
         * no streaming callback to deliver it. */
        char *outJson = nullptr;
        int status = notes_core_agent_poll(m_session.get(), &outJson);
        if (status == 1 && outJson) {
            QString resultJson = ffiStringToQString(outJson);
            QTimer::singleShot(0, this, [this, resultJson]() {
                processAgentResult(resultJson);
                m_agentBusy = false;
                emit busy_changed();
            });
        } else {
            QTimer::singleShot(0, this, [this]() {
                m_agentBusy = false;
                emit busy_changed();
            });
        }
    });
}

/* ================================================================== */
/* Q_INVOKABLE: undo_last_action                                       */
/* ================================================================== */

void AgentBridge::undo_last_action()
{
    if (m_agentBusy || !m_canUndo) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    beginOperation();

    {
        QMutexLocker lock(&m_undoMutex);
        m_undoReady = false;
    }

    QtConcurrent::run([this]() {
        char *raw = notes_core_agent_undo(m_session.get());

        QMutexLocker lock(&m_undoMutex);
        if (raw) {
            m_undoMessage = ffiStringToQString(raw);
            m_undoSuccess = true;
        } else {
            m_undoMessage = QStringLiteral("Undo returned no message");
            m_undoSuccess = false;
        }
        m_undoReady = true;

        QTimer::singleShot(0, this, [this]() {
            this->handleWorkerCompletion();
        });
    });
}

/* ================================================================== */
/* Q_INVOKABLE: cancel_operation                                       */
/* ================================================================== */

void AgentBridge::cancel_operation()
{
    if (!m_agentBusy) return;

    m_agentBusy = false;
    m_streamingText.clear();
    m_isFetching = false;
    emit streaming_text_changed();
    emit busy_changed();
    emit fetching_changed();
}

/* ================================================================== */
/* Q_INVOKABLE: fetch_models                                           */
/* ================================================================== */

void AgentBridge::fetch_models()
{
    if (m_modelsLoading) return;

    m_modelsLoading = true;
    emit models_changed();

    if (!m_netManager) {
        m_netManager = new QNetworkAccessManager(this);
    }

    /* Build the models-listing URL from provider configuration.
     * Ollama:  {endpoint}/api/tags
     * OpenAI:  {endpoint}/v1/models                                */
    QString url;
    if (m_providerType.toLower() == QStringLiteral("ollama")) {
        url = m_endpointUrl + QStringLiteral("/api/tags");
    } else {
        url = m_endpointUrl;
        if (!url.endsWith(QLatin1Char('/'))) url += QLatin1Char('/');
        url += QStringLiteral("v1/models");
    }

    qDebug("[AgentBridge] fetch_models: GET %s", url.toUtf8().constData());

    QNetworkRequest request;
    request.setUrl(QUrl(url));

    if (!m_internalApiKey.trimmed().isEmpty()) {
        request.setRawHeader("Authorization",
            QStringLiteral("Bearer %1").arg(m_internalApiKey).toUtf8());
    }

    QNetworkReply *reply = m_netManager->get(request);

    /* Qt 5.6 has no built-in transfer timeout.  A request to an
     * unreachable host hangs forever.  Start a timer that aborts the
     * reply after 5 seconds. */
    QTimer *timeoutTimer = new QTimer(this);
    timeoutTimer->setSingleShot(true);
    timeoutTimer->setInterval(5000);
    connect(timeoutTimer, &QTimer::timeout, this, [reply, timeoutTimer]() {
        reply->abort();
        timeoutTimer->deleteLater();
    });
    timeoutTimer->start();

    connect(reply, &QNetworkReply::finished, this, [this, reply, timeoutTimer]() {
        timeoutTimer->stop();
        timeoutTimer->deleteLater();
        reply->deleteLater();

        if (reply->error() != QNetworkReply::NoError) {
            QString errMsg = QStringLiteral("Model fetch failed: %1").arg(reply->errorString());
            qWarning("[AgentBridge] %s", errMsg.toUtf8().constData());
            reportError(errMsg);
            m_availableModels = QStringLiteral("[]");
            m_modelsLoading = false;
            emit models_changed();
            return;
        }

        QByteArray data = reply->readAll();
        qDebug("[AgentBridge] fetch_models: got %d bytes", data.size());

        QJsonDocument doc = QJsonDocument::fromJson(data);
        QJsonArray models;

        if (doc.isObject()) {
            QJsonObject root = doc.object();

            if (root.contains(QStringLiteral("models"))) {
                /* Ollama format */
                QJsonArray raw = root[QStringLiteral("models")].toArray();
                for (const QJsonValue &v : raw) {
                    QJsonObject entry;
                    entry[QStringLiteral("name")] =
                        v.toObject()[QStringLiteral("name")].toString();
                    models.append(entry);
                }
            } else if (root.contains(QStringLiteral("data"))) {
                /* OpenAI format */
                QJsonArray raw = root[QStringLiteral("data")].toArray();
                for (const QJsonValue &v : raw) {
                    QJsonObject entry;
                    entry[QStringLiteral("name")] =
                        v.toObject()[QStringLiteral("id")].toString();
                    models.append(entry);
                }
            }
        }

        m_availableModels = QString::fromUtf8(
            QJsonDocument(models).toJson(QJsonDocument::Compact));

        m_modelsLoading = false;
        emit models_changed();
    });
}

/* ================================================================== */
/* Q_INVOKABLE: poll_models (legacy — no-op, kept for QML compat)     */
/* ================================================================== */

bool AgentBridge::poll_models()
{
    if (m_modelsLoading) return false;
    return true;
}
