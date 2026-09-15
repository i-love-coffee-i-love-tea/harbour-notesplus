/****************************************************************************
** Meta object code from reading C++ file 'SpeechBridge.h'
**
** Created by: The Qt Meta Object Compiler version 67 (Qt 5.6.3)
**
** WARNING! All changes made in this file will be lost!
*****************************************************************************/

#include "src/bridge/SpeechBridge.h"
#include <QtCore/qbytearray.h>
#include <QtCore/qmetatype.h>
#if !defined(Q_MOC_OUTPUT_REVISION)
#error "The header file 'SpeechBridge.h' doesn't include <QObject>."
#elif Q_MOC_OUTPUT_REVISION != 67
#error "This file was generated using the moc from 5.6.3. It"
#error "cannot be used with the include files from this version of Qt."
#error "(The moc has changed too much.)"
#endif

QT_BEGIN_MOC_NAMESPACE
struct qt_meta_stringdata_SpeechBridge_t {
    QByteArrayData data[46];
    char stringdata0[753];
};
#define QT_MOC_LITERAL(idx, ofs, len) \
    Q_STATIC_BYTE_ARRAY_DATA_HEADER_INITIALIZER_WITH_OFFSET(len, \
    qptrdiff(offsetof(qt_meta_stringdata_SpeechBridge_t, stringdata0) + ofs \
        - idx * sizeof(QByteArrayData)) \
    )
static const qt_meta_stringdata_SpeechBridge_t qt_meta_stringdata_SpeechBridge = {
    {
QT_MOC_LITERAL(0, 0, 12), // "SpeechBridge"
QT_MOC_LITERAL(1, 13, 17), // "recording_changed"
QT_MOC_LITERAL(2, 31, 0), // ""
QT_MOC_LITERAL(3, 32, 20), // "transcribing_changed"
QT_MOC_LITERAL(4, 53, 19), // "downloading_changed"
QT_MOC_LITERAL(5, 73, 25), // "download_progress_changed"
QT_MOC_LITERAL(6, 99, 20), // "active_model_changed"
QT_MOC_LITERAL(7, 120, 14), // "models_changed"
QT_MOC_LITERAL(8, 135, 19), // "audio_level_changed"
QT_MOC_LITERAL(9, 155, 23), // "transcription_completed"
QT_MOC_LITERAL(10, 179, 4), // "text"
QT_MOC_LITERAL(11, 184, 18), // "download_completed"
QT_MOC_LITERAL(12, 203, 8), // "model_id"
QT_MOC_LITERAL(13, 212, 14), // "error_occurred"
QT_MOC_LITERAL(14, 227, 7), // "message"
QT_MOC_LITERAL(15, 235, 15), // "transcribe_file"
QT_MOC_LITERAL(16, 251, 4), // "path"
QT_MOC_LITERAL(17, 256, 14), // "download_model"
QT_MOC_LITERAL(18, 271, 7), // "modelId"
QT_MOC_LITERAL(19, 279, 15), // "cancel_download"
QT_MOC_LITERAL(20, 295, 12), // "delete_model"
QT_MOC_LITERAL(21, 308, 16), // "set_active_model"
QT_MOC_LITERAL(22, 325, 14), // "refresh_models"
QT_MOC_LITERAL(23, 340, 11), // "poll_worker"
QT_MOC_LITERAL(24, 352, 15), // "start_recording"
QT_MOC_LITERAL(25, 368, 14), // "stop_recording"
QT_MOC_LITERAL(26, 383, 29), // "stop_recording_and_transcribe"
QT_MOC_LITERAL(27, 413, 16), // "cancel_recording"
QT_MOC_LITERAL(28, 430, 18), // "is_model_installed"
QT_MOC_LITERAL(29, 449, 19), // "get_model_info_json"
QT_MOC_LITERAL(30, 469, 22), // "get_last_transcription"
QT_MOC_LITERAL(31, 492, 17), // "get_error_message"
QT_MOC_LITERAL(32, 510, 12), // "is_recording"
QT_MOC_LITERAL(33, 523, 15), // "is_transcribing"
QT_MOC_LITERAL(34, 539, 14), // "is_downloading"
QT_MOC_LITERAL(35, 554, 17), // "download_progress"
QT_MOC_LITERAL(36, 572, 20), // "downloading_model_id"
QT_MOC_LITERAL(37, 593, 15), // "active_model_id"
QT_MOC_LITERAL(38, 609, 21), // "available_models_json"
QT_MOC_LITERAL(39, 631, 21), // "installed_models_json"
QT_MOC_LITERAL(40, 653, 18), // "last_transcription"
QT_MOC_LITERAL(41, 672, 13), // "error_message"
QT_MOC_LITERAL(42, 686, 20), // "has_installed_models"
QT_MOC_LITERAL(43, 707, 19), // "recording_file_path"
QT_MOC_LITERAL(44, 727, 11), // "audio_level"
QT_MOC_LITERAL(45, 739, 13) // "waveform_json"

    },
    "SpeechBridge\0recording_changed\0\0"
    "transcribing_changed\0downloading_changed\0"
    "download_progress_changed\0"
    "active_model_changed\0models_changed\0"
    "audio_level_changed\0transcription_completed\0"
    "text\0download_completed\0model_id\0"
    "error_occurred\0message\0transcribe_file\0"
    "path\0download_model\0modelId\0cancel_download\0"
    "delete_model\0set_active_model\0"
    "refresh_models\0poll_worker\0start_recording\0"
    "stop_recording\0stop_recording_and_transcribe\0"
    "cancel_recording\0is_model_installed\0"
    "get_model_info_json\0get_last_transcription\0"
    "get_error_message\0is_recording\0"
    "is_transcribing\0is_downloading\0"
    "download_progress\0downloading_model_id\0"
    "active_model_id\0available_models_json\0"
    "installed_models_json\0last_transcription\0"
    "error_message\0has_installed_models\0"
    "recording_file_path\0audio_level\0"
    "waveform_json"
};
#undef QT_MOC_LITERAL

static const uint qt_meta_data_SpeechBridge[] = {

 // content:
       7,       // revision
       0,       // classname
       0,    0, // classinfo
      25,   14, // methods
      14,  182, // properties
       0,    0, // enums/sets
       0,    0, // constructors
       0,       // flags
      10,       // signalCount

 // signals: name, argc, parameters, tag, flags
       1,    0,  139,    2, 0x06 /* Public */,
       3,    0,  140,    2, 0x06 /* Public */,
       4,    0,  141,    2, 0x06 /* Public */,
       5,    0,  142,    2, 0x06 /* Public */,
       6,    0,  143,    2, 0x06 /* Public */,
       7,    0,  144,    2, 0x06 /* Public */,
       8,    0,  145,    2, 0x06 /* Public */,
       9,    1,  146,    2, 0x06 /* Public */,
      11,    1,  149,    2, 0x06 /* Public */,
      13,    1,  152,    2, 0x06 /* Public */,

 // methods: name, argc, parameters, tag, flags
      15,    1,  155,    2, 0x02 /* Public */,
      17,    1,  158,    2, 0x02 /* Public */,
      19,    0,  161,    2, 0x02 /* Public */,
      20,    1,  162,    2, 0x02 /* Public */,
      21,    1,  165,    2, 0x02 /* Public */,
      22,    0,  168,    2, 0x02 /* Public */,
      23,    0,  169,    2, 0x02 /* Public */,
      24,    0,  170,    2, 0x02 /* Public */,
      25,    0,  171,    2, 0x02 /* Public */,
      26,    0,  172,    2, 0x02 /* Public */,
      27,    0,  173,    2, 0x02 /* Public */,
      28,    1,  174,    2, 0x02 /* Public */,
      29,    1,  177,    2, 0x02 /* Public */,
      30,    0,  180,    2, 0x02 /* Public */,
      31,    0,  181,    2, 0x02 /* Public */,

 // signals: parameters
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void, QMetaType::QString,   10,
    QMetaType::Void, QMetaType::QString,   12,
    QMetaType::Void, QMetaType::QString,   14,

 // methods: parameters
    QMetaType::Void, QMetaType::QString,   16,
    QMetaType::Void, QMetaType::QString,   18,
    QMetaType::Void,
    QMetaType::Bool, QMetaType::QString,   18,
    QMetaType::Void, QMetaType::QString,   18,
    QMetaType::Void,
    QMetaType::Bool,
    QMetaType::Bool,
    QMetaType::QString,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Bool, QMetaType::QString,   18,
    QMetaType::QString, QMetaType::QString,   18,
    QMetaType::QString,
    QMetaType::QString,

 // properties: name, type, flags
      32, QMetaType::Bool, 0x00495001,
      33, QMetaType::Bool, 0x00495001,
      34, QMetaType::Bool, 0x00495001,
      35, QMetaType::Double, 0x00495001,
      36, QMetaType::QString, 0x00495001,
      37, QMetaType::QString, 0x00495001,
      38, QMetaType::QString, 0x00495001,
      39, QMetaType::QString, 0x00495001,
      40, QMetaType::QString, 0x00495001,
      41, QMetaType::QString, 0x00495001,
      42, QMetaType::Bool, 0x00495001,
      43, QMetaType::QString, 0x00495001,
      44, QMetaType::Double, 0x00495001,
      45, QMetaType::QString, 0x00495001,

 // properties: notify_signal_id
       0,
       1,
       2,
       3,
       2,
       4,
       5,
       5,
       7,
       9,
       5,
       0,
       6,
       6,

       0        // eod
};

void SpeechBridge::qt_static_metacall(QObject *_o, QMetaObject::Call _c, int _id, void **_a)
{
    if (_c == QMetaObject::InvokeMetaMethod) {
        SpeechBridge *_t = static_cast<SpeechBridge *>(_o);
        Q_UNUSED(_t)
        switch (_id) {
        case 0: _t->recording_changed(); break;
        case 1: _t->transcribing_changed(); break;
        case 2: _t->downloading_changed(); break;
        case 3: _t->download_progress_changed(); break;
        case 4: _t->active_model_changed(); break;
        case 5: _t->models_changed(); break;
        case 6: _t->audio_level_changed(); break;
        case 7: _t->transcription_completed((*reinterpret_cast< const QString(*)>(_a[1]))); break;
        case 8: _t->download_completed((*reinterpret_cast< const QString(*)>(_a[1]))); break;
        case 9: _t->error_occurred((*reinterpret_cast< const QString(*)>(_a[1]))); break;
        case 10: _t->transcribe_file((*reinterpret_cast< const QString(*)>(_a[1]))); break;
        case 11: _t->download_model((*reinterpret_cast< const QString(*)>(_a[1]))); break;
        case 12: _t->cancel_download(); break;
        case 13: { bool _r = _t->delete_model((*reinterpret_cast< const QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 14: _t->set_active_model((*reinterpret_cast< const QString(*)>(_a[1]))); break;
        case 15: _t->refresh_models(); break;
        case 16: { bool _r = _t->poll_worker();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 17: { bool _r = _t->start_recording();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 18: { QString _r = _t->stop_recording();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 19: _t->stop_recording_and_transcribe(); break;
        case 20: _t->cancel_recording(); break;
        case 21: { bool _r = _t->is_model_installed((*reinterpret_cast< const QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 22: { QString _r = _t->get_model_info_json((*reinterpret_cast< const QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 23: { QString _r = _t->get_last_transcription();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 24: { QString _r = _t->get_error_message();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        default: ;
        }
    } else if (_c == QMetaObject::IndexOfMethod) {
        int *result = reinterpret_cast<int *>(_a[0]);
        void **func = reinterpret_cast<void **>(_a[1]);
        {
            typedef void (SpeechBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::recording_changed)) {
                *result = 0;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::transcribing_changed)) {
                *result = 1;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::downloading_changed)) {
                *result = 2;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::download_progress_changed)) {
                *result = 3;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::active_model_changed)) {
                *result = 4;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::models_changed)) {
                *result = 5;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::audio_level_changed)) {
                *result = 6;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)(const QString & );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::transcription_completed)) {
                *result = 7;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)(const QString & );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::download_completed)) {
                *result = 8;
                return;
            }
        }
        {
            typedef void (SpeechBridge::*_t)(const QString & );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&SpeechBridge::error_occurred)) {
                *result = 9;
                return;
            }
        }
    }
#ifndef QT_NO_PROPERTIES
    else if (_c == QMetaObject::ReadProperty) {
        SpeechBridge *_t = static_cast<SpeechBridge *>(_o);
        Q_UNUSED(_t)
        void *_v = _a[0];
        switch (_id) {
        case 0: *reinterpret_cast< bool*>(_v) = _t->isRecording(); break;
        case 1: *reinterpret_cast< bool*>(_v) = _t->isTranscribing(); break;
        case 2: *reinterpret_cast< bool*>(_v) = _t->isDownloading(); break;
        case 3: *reinterpret_cast< double*>(_v) = _t->downloadProgress(); break;
        case 4: *reinterpret_cast< QString*>(_v) = _t->downloadingModelId(); break;
        case 5: *reinterpret_cast< QString*>(_v) = _t->activeModelId(); break;
        case 6: *reinterpret_cast< QString*>(_v) = _t->availableModelsJson(); break;
        case 7: *reinterpret_cast< QString*>(_v) = _t->installedModelsJson(); break;
        case 8: *reinterpret_cast< QString*>(_v) = _t->lastTranscription(); break;
        case 9: *reinterpret_cast< QString*>(_v) = _t->errorMessage(); break;
        case 10: *reinterpret_cast< bool*>(_v) = _t->hasInstalledModels(); break;
        case 11: *reinterpret_cast< QString*>(_v) = _t->recordingFilePath(); break;
        case 12: *reinterpret_cast< double*>(_v) = _t->audioLevel(); break;
        case 13: *reinterpret_cast< QString*>(_v) = _t->waveformJson(); break;
        default: break;
        }
    } else if (_c == QMetaObject::WriteProperty) {
    } else if (_c == QMetaObject::ResetProperty) {
    }
#endif // QT_NO_PROPERTIES
}

const QMetaObject SpeechBridge::staticMetaObject = {
    { &QObject::staticMetaObject, qt_meta_stringdata_SpeechBridge.data,
      qt_meta_data_SpeechBridge,  qt_static_metacall, Q_NULLPTR, Q_NULLPTR}
};


const QMetaObject *SpeechBridge::metaObject() const
{
    return QObject::d_ptr->metaObject ? QObject::d_ptr->dynamicMetaObject() : &staticMetaObject;
}

void *SpeechBridge::qt_metacast(const char *_clname)
{
    if (!_clname) return Q_NULLPTR;
    if (!strcmp(_clname, qt_meta_stringdata_SpeechBridge.stringdata0))
        return static_cast<void*>(const_cast< SpeechBridge*>(this));
    return QObject::qt_metacast(_clname);
}

int SpeechBridge::qt_metacall(QMetaObject::Call _c, int _id, void **_a)
{
    _id = QObject::qt_metacall(_c, _id, _a);
    if (_id < 0)
        return _id;
    if (_c == QMetaObject::InvokeMetaMethod) {
        if (_id < 25)
            qt_static_metacall(this, _c, _id, _a);
        _id -= 25;
    } else if (_c == QMetaObject::RegisterMethodArgumentMetaType) {
        if (_id < 25)
            *reinterpret_cast<int*>(_a[0]) = -1;
        _id -= 25;
    }
#ifndef QT_NO_PROPERTIES
   else if (_c == QMetaObject::ReadProperty || _c == QMetaObject::WriteProperty
            || _c == QMetaObject::ResetProperty || _c == QMetaObject::RegisterPropertyMetaType) {
        qt_static_metacall(this, _c, _id, _a);
        _id -= 14;
    } else if (_c == QMetaObject::QueryPropertyDesignable) {
        _id -= 14;
    } else if (_c == QMetaObject::QueryPropertyScriptable) {
        _id -= 14;
    } else if (_c == QMetaObject::QueryPropertyStored) {
        _id -= 14;
    } else if (_c == QMetaObject::QueryPropertyEditable) {
        _id -= 14;
    } else if (_c == QMetaObject::QueryPropertyUser) {
        _id -= 14;
    }
#endif // QT_NO_PROPERTIES
    return _id;
}

// SIGNAL 0
void SpeechBridge::recording_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 0, Q_NULLPTR);
}

// SIGNAL 1
void SpeechBridge::transcribing_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 1, Q_NULLPTR);
}

// SIGNAL 2
void SpeechBridge::downloading_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 2, Q_NULLPTR);
}

// SIGNAL 3
void SpeechBridge::download_progress_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 3, Q_NULLPTR);
}

// SIGNAL 4
void SpeechBridge::active_model_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 4, Q_NULLPTR);
}

// SIGNAL 5
void SpeechBridge::models_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 5, Q_NULLPTR);
}

// SIGNAL 6
void SpeechBridge::audio_level_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 6, Q_NULLPTR);
}

// SIGNAL 7
void SpeechBridge::transcription_completed(const QString & _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 7, _a);
}

// SIGNAL 8
void SpeechBridge::download_completed(const QString & _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 8, _a);
}

// SIGNAL 9
void SpeechBridge::error_occurred(const QString & _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 9, _a);
}
QT_END_MOC_NAMESPACE
