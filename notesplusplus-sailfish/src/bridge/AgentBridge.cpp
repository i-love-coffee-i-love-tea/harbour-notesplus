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
    m_errorMessage.clear();
    m_streamingText.clear();
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

void AgentBridge::appendAssistantMessage(const QString &text)
{
    if (text.trimmed().isEmpty()) return;
    QJsonDocument doc = QJsonDocument::fromJson(m_messagesJson.toUtf8());
    QJsonArray arr = doc.isArray() ? doc.array() : QJsonArray();

    QJsonObject asstMsg;
    asstMsg[QStringLiteral("role")]    = QStringLiteral("assistant");
    asstMsg[QStringLiteral("content")] = text;
    arr.append(asstMsg);

    m_messagesJson = QString::fromUtf8(
        QJsonDocument(arr).toJson(QJsonDocument::Compact));
    emit messages_changed();
}

/* ================================================================== */
/* Streaming callback (called from worker thread)                      */
/* ================================================================== */

static void agentStreamingTokenCallback(void *userData, const char *token, int /*isDone*/)
{
    AgentBridge *bridge = static_cast<AgentBridge*>(userData);
    if (!bridge || !token) return;

    QString tokenStr = QString::fromUtf8(token);
    QMetaObject::invokeMethod(bridge, "appendStreamingToken",
                              Qt::QueuedConnection,
                              Q_ARG(QString, tokenStr));
}

void AgentBridge::appendStreamingToken(const QString &token)
{
    m_streamingText += token;
    emit streaming_text_changed();
}

/* ================================================================== */
/* Start an agent send (streaming) — runs directly on worker thread.   */
/* ================================================================== */

void AgentBridge::sendInBackground(const QString &prompt)
{
    beginOperation();

    QtConcurrent::run([this, prompt]() {
        char *resultRaw = notes_core_agent_send_streaming_direct(
            m_session.get(),
            qstrToFFI(prompt),
            &agentStreamingTokenCallback,
            this);

        QString resultJson;
        if (resultRaw) {
            resultJson = ffiStringToQString(resultRaw);
        }

        QMetaObject::invokeMethod(this, "handleSendResult",
                                  Qt::QueuedConnection,
                                  Q_ARG(QString, resultJson));
    });
}

void AgentBridge::handleSendResult(const QString &resultJson)
{
    if (!m_agentBusy) return;

    if (!resultJson.isEmpty()) {
        processAgentResult(resultJson);
    } else {
        if (!m_streamingText.trimmed().isEmpty()) {
            appendAssistantMessage(m_streamingText);
        }
        reportError(QStringLiteral("Agent returned empty response"));
    }
    m_agentBusy = false;
    m_streamingText.clear();
    emit streaming_text_changed();
    emit busy_changed();
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

    QtConcurrent::run([this, url]() {
        char *raw = notes_core_fetch_url(qstrToFFI(url));
        QString data;
        bool ok = false;
        if (raw) {
            data = ffiStringToQString(raw);
            ok = true;
        } else {
            data = QStringLiteral("Error: fetch returned null");
        }

        QMetaObject::invokeMethod(this, "handleFetchUrlResult",
                                  Qt::QueuedConnection,
                                  Q_ARG(QString, data),
                                  Q_ARG(bool, ok));
    });
}

void AgentBridge::handleFetchUrlResult(const QString &data, bool ok)
{
    m_isFetching = false;
    emit fetching_changed();
    if (ok) {
        emit fetch_completed(data);
    } else {
        emit fetch_error(data);
    }
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
            QString err = QStringLiteral("Error: could not resolve path: ") + expanded;
            QMetaObject::invokeMethod(this, "handleReadLocalFileResult",
                                      Qt::QueuedConnection,
                                      Q_ARG(QString, err),
                                      Q_ARG(bool, false));
            return;
        }

        /* Containment check — must be inside notes_dir */
        if (!canonical.startsWith(notesDir)) {
            QString err = QStringLiteral("Error: access denied — file is outside the notes directory");
            QMetaObject::invokeMethod(this, "handleReadLocalFileResult",
                                      Qt::QueuedConnection,
                                      Q_ARG(QString, err),
                                      Q_ARG(bool, false));
            return;
        }

        /* Read the file */
        QFile file(canonical);
        if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QString err = QStringLiteral("Error reading file: ") + file.errorString();
            QMetaObject::invokeMethod(this, "handleReadLocalFileResult",
                                      Qt::QueuedConnection,
                                      Q_ARG(QString, err),
                                      Q_ARG(bool, false));
            return;
        }

        QString data = QString::fromUtf8(file.readAll());
        file.close();

        QMetaObject::invokeMethod(this, "handleReadLocalFileResult",
                                  Qt::QueuedConnection,
                                  Q_ARG(QString, data),
                                  Q_ARG(bool, true));
    });
}

void AgentBridge::handleReadLocalFileResult(const QString &data, bool ok)
{
    m_isFetching = false;
    emit fetching_changed();
    if (ok) {
        emit fetch_completed(data);
    } else {
        emit fetch_error(data);
    }
}

/* ================================================================== */
/* Q_INVOKABLE: confirm_action                                         */
/* ================================================================== */

void AgentBridge::confirm_action(bool approved)
{
    if (m_agentBusy || !m_hasPendingAction) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    beginOperation();

    int approvedInt = approved ? 1 : 0;
    QtConcurrent::run([this, approvedInt]() {
        char *resultRaw = notes_core_agent_confirm_streaming_direct(
            m_session.get(),
            approvedInt,
            &agentStreamingTokenCallback,
            this);

        QString resultJson;
        if (resultRaw) {
            resultJson = ffiStringToQString(resultRaw);
        }

        QMetaObject::invokeMethod(this, "handleConfirmResult",
                                  Qt::QueuedConnection,
                                  Q_ARG(QString, resultJson));
    });
}

void AgentBridge::handleConfirmResult(const QString &resultJson)
{
    if (!m_agentBusy) return;

    if (!resultJson.isEmpty()) {
        processAgentResult(resultJson);
    } else {
        if (!m_streamingText.trimmed().isEmpty()) {
            appendAssistantMessage(m_streamingText);
        }
        reportError(QStringLiteral("Confirmation returned empty response"));
    }
    m_agentBusy = false;
    m_streamingText.clear();
    emit streaming_text_changed();
    emit busy_changed();
}

/* ================================================================== */
/* Q_INVOKABLE: undo_last_action                                       */
/* ================================================================== */

void AgentBridge::undo_last_action()
{
    if (m_agentBusy || !m_canUndo) return;
    if (!m_session) { reportError(QStringLiteral("No agent session")); return; }

    beginOperation();

    QtConcurrent::run([this]() {
        char *resultRaw = notes_core_agent_undo_direct(m_session.get());

        QString resultJson;
        if (resultRaw) {
            resultJson = ffiStringToQString(resultRaw);
        }

        QMetaObject::invokeMethod(this, "handleUndoResult",
                                  Qt::QueuedConnection,
                                  Q_ARG(QString, resultJson));
    });
}

void AgentBridge::handleUndoResult(const QString &resultJson)
{
    if (!m_agentBusy) return;

    if (!resultJson.isEmpty()) {
        processAgentResult(resultJson);
        emit undo_completed(QStringLiteral("Action undone"));
    } else {
        reportError(QStringLiteral("Undo failed"));
    }
    m_agentBusy = false;
    emit busy_changed();
}

/* ================================================================== */
/* Q_INVOKABLE: cancel_operation                                       */
/* ================================================================== */

void AgentBridge::cancel_operation()
{
    if (!m_agentBusy) return;

    if (!m_streamingText.trimmed().isEmpty()) {
        appendAssistantMessage(m_streamingText);
    }

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
