/* SpeechBridge.h — C++ QObject bridge for Speech-to-Text (STT) and model management.
 *
 * This is the C++ port of the Rust SpeechBridge (speech_bridge.rs).
 * QML-facing property names, signal names, and method names are IDENTICAL to the Rust version.
 */

#ifndef SPEECHBRIDGE_H
#define SPEECHBRIDGE_H

#include <QObject>
#include <QString>
#include <QTimer>
#include <QtConcurrent/QtConcurrent>
#include <QJsonArray>
#include <memory>
#include <atomic>

#include "../ffi/ffi_raii.h"

/* PulseAudio in-process capture (mirrors Rust AudioRecorder). */
class PulseAudioCapture;

class SpeechBridge : public QObject
{
    Q_OBJECT

    /* ---- Properties (14) — exact names match Rust bridge ---- */
    Q_PROPERTY(bool    is_recording          READ isRecording          NOTIFY recording_changed)
    Q_PROPERTY(bool    is_transcribing       READ isTranscribing       NOTIFY transcribing_changed)
    Q_PROPERTY(bool    is_downloading        READ isDownloading        NOTIFY downloading_changed)
    Q_PROPERTY(double  download_progress     READ downloadProgress     NOTIFY download_progress_changed)
    Q_PROPERTY(QString downloading_model_id  READ downloadingModelId   NOTIFY downloading_changed)
    Q_PROPERTY(QString active_model_id       READ activeModelId        NOTIFY active_model_changed)
    Q_PROPERTY(QString available_models_json READ availableModelsJson  NOTIFY models_changed)
    Q_PROPERTY(QString installed_models_json READ installedModelsJson  NOTIFY models_changed)
    Q_PROPERTY(QString last_transcription    READ lastTranscription    NOTIFY transcription_completed)
    Q_PROPERTY(QString error_message         READ errorMessage         NOTIFY error_occurred)
    Q_PROPERTY(bool    has_installed_models  READ hasInstalledModels   NOTIFY models_changed)
    Q_PROPERTY(QString recording_file_path   READ recordingFilePath    NOTIFY recording_changed)
    Q_PROPERTY(double  audio_level           READ audioLevel           NOTIFY audio_level_changed)
    Q_PROPERTY(QString waveform_json         READ waveformJson         NOTIFY audio_level_changed)

public:
    explicit SpeechBridge(QObject *parent = nullptr);
    ~SpeechBridge() override;

    /* ---- Property accessors ---- */
    bool    isRecording()         const { return m_isRecording; }
    bool    isTranscribing()      const { return m_isTranscribing; }
    bool    isDownloading()       const { return m_isDownloading; }
    double  downloadProgress()    const { return m_downloadProgress; }
    QString downloadingModelId()  const { return m_downloadingModelId; }
    QString activeModelId()       const { return m_activeModelId; }
    QString availableModelsJson() const { return m_availableModelsJson; }
    QString installedModelsJson() const { return m_installedModelsJson; }
    QString lastTranscription()   const { return m_lastTranscription; }
    QString errorMessage()        const { return m_errorMessage; }
    bool    hasInstalledModels()  const { return m_hasInstalledModels; }
    QString recordingFilePath()   const { return m_recordingFilePath; }
    double  audioLevel()          const { return m_audioLevel; }
    QString waveformJson()        const { return m_waveformJson; }

    /* ---- Q_INVOKABLE methods (15) — exact names match Rust bridge ---- */
    Q_INVOKABLE void    transcribe_file(const QString &path);
    Q_INVOKABLE void    download_model(const QString &modelId);
    Q_INVOKABLE void    cancel_download();
    Q_INVOKABLE bool    delete_model(const QString &modelId);
    Q_INVOKABLE void    set_active_model(const QString &modelId);
    Q_INVOKABLE void    refresh_models();
    Q_INVOKABLE bool    poll_worker();
    Q_INVOKABLE bool    start_recording();
    Q_INVOKABLE QString stop_recording();
    Q_INVOKABLE void    stop_recording_and_transcribe();
    Q_INVOKABLE void    cancel_recording();
    Q_INVOKABLE bool    is_model_installed(const QString &modelId) const;
    Q_INVOKABLE QString get_model_info_json(const QString &modelId) const;
    Q_INVOKABLE QString get_last_transcription() const { return m_lastTranscription; }
    Q_INVOKABLE QString get_error_message() const { return m_errorMessage; }

signals:
    /* ---- Signals (10) — exact names match Rust bridge ---- */
    void recording_changed();
    void transcribing_changed();
    void downloading_changed();
    void download_progress_changed();
    void active_model_changed();
    void models_changed();
    void audio_level_changed();
    void transcription_completed(const QString &text);
    void download_completed(const QString &model_id);
    void error_occurred(const QString &message);

private:
    /* helpers */
    void reportError(const QString &msg);
    QString modelFilePath(const QString &modelId) const;
    void beginTranscription(const QString &path);

    /* ---- Property storage ---- */
    bool    m_isRecording         = false;
    bool    m_isTranscribing      = false;
    bool    m_isDownloading       = false;
    double  m_downloadProgress    = 0.0;
    QString m_downloadingModelId;
    QString m_activeModelId;
    QString m_availableModelsJson;
    QString m_installedModelsJson;
    QString m_lastTranscription;
    QString m_errorMessage;
    bool    m_hasInstalledModels  = false;
    QString m_recordingFilePath;
    double  m_audioLevel          = 0.0;
    QString m_waveformJson;

    /* ---- Internal state ---- */
    QString m_modelsDir;                           // computed from AppPaths::data_dir + "/models/stt"

    /* Download state */
    SttDownloadPtr              m_download;        // RAII FFI handle (nullptr when idle)
    std::atomic<bool>           m_cancelFlag{false};
    std::atomic<bool>           m_downloadDone{false};
    QString                     m_downloadResult;
    bool                        m_downloadSuccess  = false;
    double                      m_progressSlot     = 0.0;

    /* Transcription state */
    std::atomic<bool>           m_transcribeDone{false};
    QString                     m_transcribeResult;
    bool                        m_transcribeSuccess = false;

    /* Poll timer (drives poll_worker from QML or internally) */
    QTimer                     *m_pollTimer        = nullptr;

    /* Audio capture (PulseAudio in-process, mirrors Rust AudioRecorder) */
    std::unique_ptr<PulseAudioCapture> m_capture;
};

#endif /* SPEECHBRIDGE_H */
