/* SpeechBridge.cpp — C++ QObject bridge for Speech-to-Text (STT) and model management.
 *
 * C++ port of speech_bridge.rs. QML-facing API is IDENTICAL.
 */

#include "SpeechBridge.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QDebug>
#include <QtConcurrent/QtConcurrent>

#include <cstring>
#include <cmath>
#include <atomic>
#include <thread>
#include <mutex>

/* ======================================================================== */
/* PulseAudio in-process capture (C++ port of Rust audio_recorder.rs)       */
/* ======================================================================== */

#include <dlfcn.h>
#include <cstdint>

static constexpr int    PA_SAMPLE_S16LE = 3;
static constexpr int    PA_STREAM_RECORD = 2;
static constexpr uint32_t SAMPLE_RATE   = 16000;
static constexpr uint16_t CHANNELS      = 1;
static constexpr int    BUFFER_SIZE     = 2048; // ~64 ms at 16 kHz 16-bit mono

/* PulseAudio Simple API function types */
using PaSimpleNewFn  = void*(*)(const char*, const char*, int, const char*,
                                const char*, const void*, const void*,
                                const void*, int*);
using PaSimpleReadFn = int(*)(void*, void*, size_t, int*);
using PaSimpleFreeFn = void(*)(void*);

/* PA sample spec struct (matches C API layout) */
struct PaSampleSpec {
    int      format;
    uint32_t rate;
    uint8_t  channels;
};

/*
 * PulseAudioCapture — wraps dlopen of libpulse-simple.so and runs a
 * background recording thread.  Provides live RMS level and a 7-element
 * waveform history, matching the Rust AudioRecorder.
 */
class PulseAudioCapture
{
public:
    PulseAudioCapture()
        : m_paLib(nullptr)
        , m_paNew(nullptr)
        , m_paRead(nullptr)
        , m_paFree(nullptr)
    {
        loadPulseAudio();
    }

    ~PulseAudioCapture() { cancel(); unloadPulseAudio(); }

    bool isAvailable() const { return m_paNew && m_paRead && m_paFree; }

    bool isRecording() const { return m_recording.load(); }

    // Start recording to |outputPath|. Returns true on success.
    bool start(const QString &outputPath)
    {
        if (m_recording.load()) return false;
        if (!isAvailable()) return false;

        // Reset level state
        {
            std::lock_guard<std::mutex> lk(m_levelMutex);
            m_level = 0.0;
            m_peak  = 0.0;
            m_history = QJsonArray();
            for (int i = 0; i < 7; ++i) m_history.append(0.0);
        }

        // Remove stale file
        QFile::remove(outputPath);

        // Open PulseAudio recording stream
        PaSampleSpec spec;
        spec.format   = PA_SAMPLE_S16LE;
        spec.rate     = SAMPLE_RATE;
        spec.channels = static_cast<uint8_t>(CHANNELS);

        int paErr = 0;
        void *simple = m_paNew(nullptr, "Notes Plus", PA_STREAM_RECORD, nullptr,
                               "STT Voice Input", &spec, nullptr, nullptr, &paErr);
        if (!simple) {
            qWarning() << "[SpeechBridge] PulseAudio stream open failed, code" << paErr;
            return false;
        }

        m_stopFlag.store(false);
        m_recording.store(true);
        m_outputPath = outputPath;

        m_thread = std::thread([this, simple, outputPath]() {
            recordThread(simple, outputPath);
        });

        return true;
    }

    // Stop recording gracefully. Returns the WAV path.
    QString stop()
    {
        if (!m_recording.load()) return m_outputPath;

        m_stopFlag.store(true);
        if (m_thread.joinable()) m_thread.join();
        m_recording.store(false);

        // Reset level state
        {
            std::lock_guard<std::mutex> lk(m_levelMutex);
            m_level = 0.0;
            m_peak  = 0.0;
            m_history = QJsonArray();
            for (int i = 0; i < 7; ++i) m_history.append(0.0);
        }

        return m_outputPath;
    }

    // Cancel recording and delete partial file.
    void cancel()
    {
        m_stopFlag.store(true);
        if (m_thread.joinable()) m_thread.join();
        m_recording.store(false);
        QFile::remove(m_outputPath);

        std::lock_guard<std::mutex> lk(m_levelMutex);
        m_level = 0.0;
        m_peak  = 0.0;
        m_history = QJsonArray();
        for (int i = 0; i < 7; ++i) m_history.append(0.0);
    }

    // Read current audio level (0..1) and waveform history.
    std::pair<double, QJsonArray> getLevelAndHistory() const
    {
        std::lock_guard<std::mutex> lk(m_levelMutex);
        return { m_level, m_history };
    }

private:
    void loadPulseAudio()
    {
        const char *libNames[] = {
            "libpulse-simple.so.0",
            "libpulse-simple.so"
        };
        void *handle = nullptr;
        for (const char *name : libNames) {
            handle = dlopen(name, RTLD_LAZY | RTLD_LOCAL);
            if (handle) break;
        }
        if (!handle) return;

        m_paLib   = handle;
        m_paNew   = reinterpret_cast<PaSimpleNewFn>(dlsym(handle, "pa_simple_new"));
        m_paRead  = reinterpret_cast<PaSimpleReadFn>(dlsym(handle, "pa_simple_read"));
        m_paFree  = reinterpret_cast<PaSimpleFreeFn>(dlsym(handle, "pa_simple_free"));
    }

    void unloadPulseAudio()
    {
        if (m_paLib) { dlclose(m_paLib); m_paLib = nullptr; }
    }

    // Write 44-byte WAV header with data_len=0 (to be finalized later).
    static bool writeWavHeader(FILE *f, uint32_t dataLen)
    {
        uint32_t riffSize = 36 + dataLen;
        uint32_t byteRate = SAMPLE_RATE * CHANNELS * 2;
        uint16_t blockAlign = CHANNELS * 2;
        uint16_t bitsPerSample = 16;

        auto writeU32 = [f](uint32_t v) { fwrite(&v, 4, 1, f); };
        auto writeU16 = [f](uint16_t v) { fwrite(&v, 2, 1, f); };

        fwrite("RIFF", 1, 4, f);
        writeU32(riffSize);
        fwrite("WAVE", 1, 4, f);
        fwrite("fmt ", 1, 4, f);
        writeU32(16);               // subchunk1 size
        writeU16(1);                // PCM format
        writeU16(CHANNELS);
        writeU32(SAMPLE_RATE);
        writeU32(byteRate);
        writeU16(blockAlign);
        writeU16(bitsPerSample);
        fwrite("data", 1, 4, f);
        writeU32(dataLen);
        return true;
    }

    // Finalize WAV header at positions 4 and 40.
    static void finalizeWavHeader(FILE *f, uint32_t dataLen)
    {
        uint32_t riffSize = 36 + dataLen;
        fseek(f, 4, SEEK_SET);
        fwrite(&riffSize, 4, 1, f);
        fseek(f, 40, SEEK_SET);
        fwrite(&dataLen, 4, 1, f);
        fflush(f);
    }

    void recordThread(void *simple, const QString &outputPath)
    {
        FILE *f = fopen(outputPath.toUtf8().constData(), "wb");
        if (!f) {
            m_paFree(simple);
            m_recording.store(false);
            return;
        }

        writeWavHeader(f, 0);

        uint8_t buffer[BUFFER_SIZE];
        uint64_t totalBytes = 0;
        int paErr = 0;

        while (!m_stopFlag.load()) {
            int rc = m_paRead(simple, buffer, BUFFER_SIZE, &paErr);
            if (rc < 0) {
                qWarning() << "[SpeechBridge] PulseAudio read error, code" << paErr;
                break;
            }
            if (fwrite(buffer, 1, BUFFER_SIZE, f) != static_cast<size_t>(BUFFER_SIZE))
                break;
            totalBytes += BUFFER_SIZE;

            // Live RMS and peak volume calculation (mirrors Rust)
            int sampleCount = BUFFER_SIZE / 2;
            if (sampleCount > 0) {
                double sumSquares = 0.0;
                int16_t peak = 0;
                for (int i = 0; i < BUFFER_SIZE; i += 2) {
                    int16_t sample;
                    memcpy(&sample, buffer + i, 2);
                    int16_t absSample = sample < 0 ? (sample == -32768 ? 32767 : -sample) : sample;
                    if (absSample > peak) peak = absSample;
                    double norm = static_cast<double>(sample) / 32768.0;
                    sumSquares += norm * norm;
                }
                double rms = std::sqrt(sumSquares / sampleCount);
                double boosted = std::min(rms * 4.5, 1.0);

                std::lock_guard<std::mutex> lk(m_levelMutex);
                m_level = boosted;
                m_peak  = std::min(static_cast<double>(peak) / 32768.0, 1.0);
                if (m_history.size() >= 7) m_history.removeFirst();
                m_history.append(boosted);
            }
        }

        m_paFree(simple);
        finalizeWavHeader(f, static_cast<uint32_t>(totalBytes));
        fclose(f);
    }

    /* dlopen handles */
    void           *m_paLib;
    PaSimpleNewFn   m_paNew;
    PaSimpleReadFn  m_paRead;
    PaSimpleFreeFn  m_paFree;

    /* recording state */
    std::atomic<bool>   m_stopFlag{false};
    std::atomic<bool>   m_recording{false};
    QString             m_outputPath;
    std::thread         m_thread;

    /* audio level state */
    mutable std::mutex  m_levelMutex;
    double              m_level   = 0.0;
    double              m_peak    = 0.0;
    QJsonArray          m_history;
};

/* ======================================================================== */
/* SpeechBridge implementation                                              */
/* ======================================================================== */

static const QString DEFAULT_WAVEFORM_JSON = QStringLiteral("[0,0,0,0,0,0,0]");

SpeechBridge::SpeechBridge(QObject *parent)
    : QObject(parent)
    , m_capture(std::make_unique<PulseAudioCapture>())
{
    /* ---- Compute models_dir from AppPaths ---- */
    AppPathsPtr paths(notes_core_app_paths_new());
    FfiString dataDirRaw(notes_core_app_paths_data_dir(paths.get()));
    QString dataDir = dataDirRaw ? QString::fromUtf8(dataDirRaw.get()) : QString();
    if (dataDir.isEmpty()) {
        dataDir = QDir::homePath() + QStringLiteral("/.local/share/harbour-notesplus");
    }
    m_modelsDir = dataDir + QStringLiteral("/models/stt");
    QDir().mkpath(m_modelsDir);

    /* ---- Initialize waveform ---- */
    m_waveformJson = DEFAULT_WAVEFORM_JSON;

    /* ---- Populate initial catalog (mirrors Rust Default::default()) ---- */
    const char *modelsDirC = m_modelsDir.toUtf8().constData();
    {
        FfiString catalogRaw(notes_core_stt_model_catalog_json(modelsDirC, nullptr));
        QJsonDocument doc = QJsonDocument::fromJson(
            catalogRaw ? QByteArray(catalogRaw.get()) : QByteArray("[]"));
        QJsonArray catalog = doc.array();

        // Find first installed model to set as active
        for (const QJsonValue &v : catalog) {
            QJsonObject m = v.toObject();
            if (m.value(QStringLiteral("is_installed")).toBool()) {
                m_activeModelId = m.value(QStringLiteral("id")).toString();
                break;
            }
        }

        // Refresh catalog with active model set
        if (!m_activeModelId.isEmpty()) {
            FfiString catRaw(notes_core_stt_model_catalog_json(
                modelsDirC, m_activeModelId.toUtf8().constData()));
            doc = QJsonDocument::fromJson(catRaw ? QByteArray(catRaw.get()) : QByteArray("[]"));
            catalog = doc.array();
        }

        m_availableModelsJson = QString::fromUtf8(doc.toJson(QJsonDocument::Compact));

        QJsonArray installed;
        for (const QJsonValue &v : catalog) {
            if (v.toObject().value(QStringLiteral("is_installed")).toBool())
                installed.append(v);
        }
        m_installedModelsJson = QString::fromUtf8(
            QJsonDocument(installed).toJson(QJsonDocument::Compact));
        m_hasInstalledModels = !installed.isEmpty();
    }

    /* ---- Poll timer (drives poll_worker from QML or internally) ---- */
    m_pollTimer = new QTimer(this);
    m_pollTimer->setInterval(100); // 10 Hz, matches typical QML Timer interval
    connect(m_pollTimer, &QTimer::timeout, this, [this]() { poll_worker(); });
}

SpeechBridge::~SpeechBridge()
{
    m_pollTimer->stop();

    // Cancel any in-progress download
    if (m_download) {
        notes_core_stt_download_cancel(m_download.get());
        m_download.reset();
    }

    // Cancel recording
    if (m_capture) m_capture->cancel();
}

/* ======================================================================== */
/* Q_INVOKABLE methods                                                      */
/* ======================================================================== */

void SpeechBridge::refresh_models()
{
    QByteArray modelsDirUtf8 = m_modelsDir.toUtf8();
    const char *md = modelsDirUtf8.constData();

    QByteArray activeUtf8 = m_activeModelId.toUtf8();
    const char *active = m_activeModelId.isEmpty() ? nullptr : activeUtf8.constData();

    // Get initial catalog
    FfiString catRaw(notes_core_stt_model_catalog_json(md, active));
    QJsonDocument doc = QJsonDocument::fromJson(catRaw ? QByteArray(catRaw.get()) : QByteArray("[]"));
    QJsonArray catalog = doc.array();

    // Check if current active model is still installed
    bool activeInstalled = false;
    for (const QJsonValue &v : catalog) {
        QJsonObject m = v.toObject();
        if (m.value(QStringLiteral("id")).toString() == m_activeModelId &&
            m.value(QStringLiteral("is_installed")).toBool()) {
            activeInstalled = true;
            break;
        }
    }

    QJsonArray finalCatalog;
    if (!activeInstalled) {
        // Try to pick the first installed model
        QString firstInstalled;
        for (const QJsonValue &v : catalog) {
            QJsonObject m = v.toObject();
            if (m.value(QStringLiteral("is_installed")).toBool()) {
                firstInstalled = m.value(QStringLiteral("id")).toString();
                break;
            }
        }

        if (!firstInstalled.isEmpty()) {
            if (m_activeModelId != firstInstalled) {
                m_activeModelId = firstInstalled;
                emit active_model_changed();
            }
            // Refresh catalog with new active model
            QByteArray newActiveUtf8 = m_activeModelId.toUtf8();
            FfiString catRaw2(notes_core_stt_model_catalog_json(md, newActiveUtf8.constData()));
            doc = QJsonDocument::fromJson(catRaw2 ? QByteArray(catRaw2.get()) : QByteArray("[]"));
            finalCatalog = doc.array();
        } else {
            if (!m_activeModelId.isEmpty()) {
                m_activeModelId.clear();
                emit active_model_changed();
            }
            finalCatalog = catalog;
        }
    } else {
        finalCatalog = catalog;
    }

    m_availableModelsJson = QString::fromUtf8(
        QJsonDocument(finalCatalog).toJson(QJsonDocument::Compact));

    QJsonArray installed;
    for (const QJsonValue &v : finalCatalog) {
        if (v.toObject().value(QStringLiteral("is_installed")).toBool())
            installed.append(v);
    }
    m_installedModelsJson = QString::fromUtf8(
        QJsonDocument(installed).toJson(QJsonDocument::Compact));
    m_hasInstalledModels = !installed.isEmpty();

    emit models_changed();
}

void SpeechBridge::set_active_model(const QString &modelId)
{
    if (m_activeModelId == modelId) return;
    m_activeModelId = modelId;
    emit active_model_changed();
    refresh_models();
}

void SpeechBridge::download_model(const QString &modelId)
{
    if (m_isDownloading) cancel_download();

    m_isDownloading = true;
    m_downloadingModelId = modelId;
    m_downloadProgress = 0.0;
    m_errorMessage.clear();
    emit downloading_changed();
    emit download_progress_changed();

    // Reset result slots
    m_downloadDone.store(false);
    m_downloadResult.clear();
    m_downloadSuccess = false;

    // Start the download via FFI
    SttDownloadPtr handle(notes_core_stt_download_start(
        m_modelsDir.toUtf8().constData(),
        modelId.toUtf8().constData()));

    if (!handle) {
        reportError(QStringLiteral("Failed to start download for model: ") + modelId);
        m_isDownloading = false;
        m_downloadingModelId.clear();
        emit downloading_changed();
        return;
    }

    // Move handle to the poll-managed slot; the poll_worker will own it.
    m_download = std::move(handle);
    m_progressSlot = 0.0;

    // Start poll timer if not already running
    if (!m_pollTimer->isActive()) m_pollTimer->start();
}

void SpeechBridge::cancel_download()
{
    m_cancelFlag.store(true);
    if (m_download) {
        notes_core_stt_download_cancel(m_download.get());
    }
}

bool SpeechBridge::delete_model(const QString &modelId)
{
    if (m_isDownloading && m_downloadingModelId == modelId) {
        cancel_download();
    }

    int rc = notes_core_stt_model_delete(
        m_modelsDir.toUtf8().constData(),
        modelId.toUtf8().constData());

    refresh_models();
    return rc == 0;
}

void SpeechBridge::transcribe_file(const QString &path)
{
    if (m_isTranscribing) return;

    m_isTranscribing = true;
    m_errorMessage.clear();
    emit transcribing_changed();
    beginTranscription(path);
}

void SpeechBridge::beginTranscription(const QString &path)
{
    QString trimmedPath = path.trimmed();
    if (trimmedPath.startsWith(QStringLiteral("file://")))
        trimmedPath = trimmedPath.mid(7);

    if (trimmedPath.isEmpty()) {
        qWarning() << "[SpeechBridge] begin_transcription: Audio path is empty";
        reportError(QStringLiteral("Audio path is empty"));
        m_isTranscribing = false;
        emit transcribing_changed();
        return;
    }

    QFileInfo fi(trimmedPath);
    if (!fi.isFile()) {
        qWarning() << "[SpeechBridge] begin_transcription: Audio file not found:" << trimmedPath;
        reportError(QStringLiteral("Audio file not found: ") + trimmedPath);
        m_isTranscribing = false;
        emit transcribing_changed();
        return;
    }

    // Determine which model to use
    QString modelId = m_activeModelId;
    if (modelId.isEmpty()) {
        // Try to find an installed model from the catalog
        FfiString catRaw(notes_core_stt_model_catalog_json(
            m_modelsDir.toUtf8().constData(), nullptr));
        QJsonDocument doc = QJsonDocument::fromJson(
            catRaw ? QByteArray(catRaw.get()) : QByteArray("[]"));
        for (const QJsonValue &v : doc.array()) {
            QJsonObject m = v.toObject();
            if (m.value(QStringLiteral("is_installed")).toBool()) {
                modelId = m.value(QStringLiteral("id")).toString();
                m_activeModelId = modelId;
                emit active_model_changed();
                break;
            }
        }
    }

    if (modelId.isEmpty()) {
        reportError(QStringLiteral("No speech model installed. Please download a model first."));
        m_isTranscribing = false;
        emit transcribing_changed();
        return;
    }

    QString mpPath = modelFilePath(modelId);
    if (!QFileInfo::exists(mpPath)) {
        reportError(QStringLiteral("Model file for '") + modelId +
                    QStringLiteral("' not found on disk. Please re-download the model."));
        m_isTranscribing = false;
        emit transcribing_changed();
        return;
    }

    qDebug() << "[SpeechBridge] begin_transcription: spawning thread for model="
             << modelId << "audio=" << trimmedPath;

    // Reset result slots
    m_transcribeDone.store(false);
    m_transcribeResult.clear();
    m_transcribeSuccess = false;

    // Run transcription in background via QtConcurrent
    QByteArray modelPathUtf8 = mpPath.toUtf8();
    QByteArray audioPathUtf8 = trimmedPath.toUtf8();

    QFuture<void> future = QtConcurrent::run([this, modelPathUtf8, audioPathUtf8]() {
        FfiString result(notes_core_stt_transcribe(
            modelPathUtf8.constData(),
            audioPathUtf8.constData()));
        if (result) {
            m_transcribeResult = QString::fromUtf8(result.get());
            m_transcribeSuccess = true;
        } else {
            m_transcribeResult = QStringLiteral("Transcription returned null");
            m_transcribeSuccess = false;
        }
        m_transcribeDone.store(true);

        QMetaObject::invokeMethod(this, [this]() {
            this->poll_worker();
        }, Qt::QueuedConnection);
    });

    // Keep poll timer running while transcribing
    if (!m_pollTimer->isActive()) m_pollTimer->start();
}

bool SpeechBridge::poll_worker()
{
    bool stateChanged = false;

    /* ---- Poll download progress ---- */
    if (m_isDownloading && m_download) {
        double outProgress = 0.0;
        char *outResult = nullptr;
        int status = notes_core_stt_download_poll(
            m_download.get(), &outProgress, &outResult);

        if (std::abs(outProgress - m_downloadProgress) > 0.01) {
            m_downloadProgress = outProgress;
            emit download_progress_changed();
            stateChanged = true;
        }

        if (status != 0) { // 1=done, -1=error
            m_isDownloading = false;
            m_downloadingModelId.clear();
            emit downloading_changed();

            if (status == 1 && outResult) {
                // success
                m_downloadProgress = 100.0;
                emit download_progress_changed();
                refresh_models();
                if (m_activeModelId.isEmpty())
                    set_active_model(QString::fromUtf8(outResult));
                emit download_completed(QString::fromUtf8(outResult));
            } else if (status == 1) {
                // done but no result id — use the model id we started with
                m_downloadProgress = 100.0;
                emit download_progress_changed();
                refresh_models();
            } else {
                // error
                QString errMsg = outResult
                    ? QString::fromUtf8(outResult)
                    : QStringLiteral("Download failed");
                reportError(errMsg);
            }

            notes_core_free_string(outResult);
            m_download.reset(); // release FFI handle
            stateChanged = true;
        }
    }

    /* ---- Poll transcription completion ---- */
    if (m_isTranscribing && m_transcribeDone.load()) {
        m_isTranscribing = false;
        emit transcribing_changed();

        if (m_transcribeSuccess) {
            // Clean up whisper placeholders (mirrors Rust)
            QString cleaned = m_transcribeResult;
            cleaned.replace(QStringLiteral("[BLANK_AUDIO]"), QString());
            cleaned.replace(QStringLiteral("[MUSIC]"), QString());
            cleaned = cleaned.trimmed();

            qDebug() << "[SpeechBridge] Transcription completed:"
                     << "'" << m_transcribeResult << "' (cleaned: '" << cleaned << "')";

            if (cleaned.isEmpty()) {
                m_lastTranscription.clear();
                emit transcription_completed(QString());
            } else {
                m_lastTranscription = cleaned;
                emit transcription_completed(cleaned);
            }
        } else {
            reportError(m_transcribeResult);
        }

        stateChanged = true;
    }

    /* ---- Poll live recording volume & waveform ---- */
    if (m_isRecording && m_capture) {
        auto [level, wfJson] = m_capture->getLevelAndHistory();
        QString wfStr = QString::fromUtf8(
            QJsonDocument(wfJson).toJson(QJsonDocument::Compact));

        if (std::abs(m_audioLevel - level) > 0.001 || m_waveformJson != wfStr) {
            m_audioLevel = level;
            m_waveformJson = wfStr;
            emit audio_level_changed();
            stateChanged = true;
        }
    }

    /* ---- Stop poll timer when idle ---- */
    bool anyActive = m_isDownloading || m_isTranscribing || m_isRecording;
    if (!anyActive && !stateChanged && m_pollTimer->isActive()) {
        m_pollTimer->stop();
    }

    return stateChanged || anyActive;
}

bool SpeechBridge::start_recording()
{
    if (m_isRecording) return true;

    if (m_isTranscribing) {
        reportError(QStringLiteral("Cannot start recording while transcription is in progress"));
        return false;
    }

    if (!m_capture || !m_capture->isAvailable()) {
        reportError(QStringLiteral("Audio recording not available (PulseAudio not loaded)"));
        return false;
    }

    // Compute output path (matches Rust: XDG_CACHE_HOME/harbour-notesplus/stt-recording.wav)
    QString cacheDir = QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
    if (cacheDir.isEmpty())
        cacheDir = QDir::homePath() + QStringLiteral("/.cache/harbour-notesplus");
    else
        cacheDir += QStringLiteral("/harbour-notesplus");
    QDir().mkpath(cacheDir);
    QString outputPath = cacheDir + QStringLiteral("/stt-recording.wav");

    if (m_capture->start(outputPath)) {
        m_recordingFilePath = outputPath;
        m_isRecording = true;
        m_audioLevel = 0.0;
        m_waveformJson = DEFAULT_WAVEFORM_JSON;
        m_errorMessage.clear();
        emit recording_changed();
        emit audio_level_changed();
        if (!m_pollTimer->isActive()) m_pollTimer->start();
        return true;
    }

    m_isRecording = false;
    m_recordingFilePath.clear();
    m_audioLevel = 0.0;
    m_waveformJson = DEFAULT_WAVEFORM_JSON;
    reportError(QStringLiteral("Failed to start audio recording"));
    emit recording_changed();
    emit audio_level_changed();
    return false;
}

QString SpeechBridge::stop_recording()
{
    if (!m_isRecording && (!m_capture || !m_capture->isRecording()))
        return m_recordingFilePath;

    m_audioLevel = 0.0;
    m_waveformJson = DEFAULT_WAVEFORM_JSON;
    emit audio_level_changed();

    QString path = m_capture->stop();
    m_recordingFilePath = path;
    m_isRecording = false;
    emit recording_changed();

    return path;
}

void SpeechBridge::stop_recording_and_transcribe()
{
    qDebug() << "[SpeechBridge] stop_recording_and_transcribe called";

    // Set transcribing flag before stopping so the poll timer
    // never sees both is_recording and is_transcribing as false.
    m_isTranscribing = true;
    m_errorMessage.clear();
    emit transcribing_changed();

    QString path = stop_recording();
    qDebug() << "[SpeechBridge] stop_recording returned path:" << path;

    if (path.isEmpty()) {
        m_isTranscribing = false;
        emit transcribing_changed();
        return;
    }
    beginTranscription(path);
}

void SpeechBridge::cancel_recording()
{
    if (m_capture) m_capture->cancel();

    m_audioLevel = 0.0;
    m_waveformJson = DEFAULT_WAVEFORM_JSON;
    emit audio_level_changed();

    if (m_isRecording || !m_recordingFilePath.isEmpty()) {
        m_isRecording = false;
        m_recordingFilePath.clear();
        emit recording_changed();
    }
}

bool SpeechBridge::is_model_installed(const QString &modelId) const
{
    return QFileInfo::exists(modelFilePath(modelId));
}

QString SpeechBridge::get_model_info_json(const QString &modelId) const
{
    QByteArray modelsDirUtf8 = m_modelsDir.toUtf8();
    QByteArray activeUtf8 = m_activeModelId.toUtf8();
    const char *active = m_activeModelId.isEmpty() ? nullptr : activeUtf8.constData();

    FfiString catRaw(notes_core_stt_model_catalog_json(modelsDirUtf8.constData(), active));
    QJsonDocument doc = QJsonDocument::fromJson(
        catRaw ? QByteArray(catRaw.get()) : QByteArray("[]"));

    for (const QJsonValue &v : doc.array()) {
        QJsonObject m = v.toObject();
        if (m.value(QStringLiteral("id")).toString() == modelId) {
            return QString::fromUtf8(
                QJsonDocument(m).toJson(QJsonDocument::Compact));
        }
    }
    return QString();
}

/* ======================================================================== */
/* Private helpers                                                          */
/* ======================================================================== */

void SpeechBridge::reportError(const QString &msg)
{
    m_errorMessage = msg;
    emit error_occurred(msg);
}

QString SpeechBridge::modelFilePath(const QString &modelId) const
{
    // Mirrors Rust: models_dir / "{model_id}.bin"
    return m_modelsDir + QStringLiteral("/") + modelId + QStringLiteral(".bin");
}
