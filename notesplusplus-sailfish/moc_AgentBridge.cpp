/****************************************************************************
** Meta object code from reading C++ file 'AgentBridge.h'
**
** Created by: The Qt Meta Object Compiler version 67 (Qt 5.6.3)
**
** WARNING! All changes made in this file will be lost!
*****************************************************************************/

#include "src/bridge/AgentBridge.h"
#include <QtCore/qbytearray.h>
#include <QtCore/qmetatype.h>
#if !defined(Q_MOC_OUTPUT_REVISION)
#error "The header file 'AgentBridge.h' doesn't include <QObject>."
#elif Q_MOC_OUTPUT_REVISION != 67
#error "This file was generated using the moc from 5.6.3. It"
#error "cannot be used with the include files from this version of Qt."
#error "(The moc has changed too much.)"
#endif

QT_BEGIN_MOC_NAMESPACE
struct qt_meta_stringdata_AgentBridge_t {
    QByteArrayData data[75];
    char stringdata0[1021];
};
#define QT_MOC_LITERAL(idx, ofs, len) \
    Q_STATIC_BYTE_ARRAY_DATA_HEADER_INITIALIZER_WITH_OFFSET(len, \
    qptrdiff(offsetof(qt_meta_stringdata_AgentBridge_t, stringdata0) + ofs \
        - idx * sizeof(QByteArrayData)) \
    )
static const qt_meta_stringdata_AgentBridge_t qt_meta_stringdata_AgentBridge = {
    {
QT_MOC_LITERAL(0, 0, 11), // "AgentBridge"
QT_MOC_LITERAL(1, 12, 12), // "busy_changed"
QT_MOC_LITERAL(2, 25, 0), // ""
QT_MOC_LITERAL(3, 26, 16), // "messages_changed"
QT_MOC_LITERAL(4, 43, 22), // "pending_action_changed"
QT_MOC_LITERAL(5, 66, 18), // "undo_state_changed"
QT_MOC_LITERAL(6, 85, 25), // "last_created_note_changed"
QT_MOC_LITERAL(7, 111, 14), // "config_changed"
QT_MOC_LITERAL(8, 126, 14), // "error_occurred"
QT_MOC_LITERAL(9, 141, 7), // "message"
QT_MOC_LITERAL(10, 149, 17), // "response_finished"
QT_MOC_LITERAL(11, 167, 7), // "content"
QT_MOC_LITERAL(12, 175, 14), // "undo_completed"
QT_MOC_LITERAL(13, 190, 22), // "streaming_text_changed"
QT_MOC_LITERAL(14, 213, 16), // "fetching_changed"
QT_MOC_LITERAL(15, 230, 15), // "fetch_completed"
QT_MOC_LITERAL(16, 246, 6), // "result"
QT_MOC_LITERAL(17, 253, 11), // "fetch_error"
QT_MOC_LITERAL(18, 265, 14), // "models_changed"
QT_MOC_LITERAL(19, 280, 9), // "configure"
QT_MOC_LITERAL(20, 290, 8), // "provider"
QT_MOC_LITERAL(21, 299, 3), // "url"
QT_MOC_LITERAL(22, 303, 5), // "model"
QT_MOC_LITERAL(23, 309, 3), // "key"
QT_MOC_LITERAL(24, 313, 7), // "timeout"
QT_MOC_LITERAL(25, 321, 9), // "auto_read"
QT_MOC_LITERAL(26, 331, 11), // "auto_create"
QT_MOC_LITERAL(27, 343, 12), // "require_edit"
QT_MOC_LITERAL(28, 356, 17), // "allow_self_signed"
QT_MOC_LITERAL(29, 374, 11), // "allow_fetch"
QT_MOC_LITERAL(30, 386, 13), // "reset_session"
QT_MOC_LITERAL(31, 400, 16), // "context_filename"
QT_MOC_LITERAL(32, 417, 15), // "context_content"
QT_MOC_LITERAL(33, 433, 13), // "extra_context"
QT_MOC_LITERAL(34, 447, 11), // "send_prompt"
QT_MOC_LITERAL(35, 459, 4), // "text"
QT_MOC_LITERAL(36, 464, 12), // "run_template"
QT_MOC_LITERAL(37, 477, 11), // "template_id"
QT_MOC_LITERAL(38, 489, 10), // "input_text"
QT_MOC_LITERAL(39, 500, 22), // "run_custom_instruction"
QT_MOC_LITERAL(40, 523, 11), // "instruction"
QT_MOC_LITERAL(41, 535, 11), // "import_text"
QT_MOC_LITERAL(42, 547, 11), // "source_text"
QT_MOC_LITERAL(43, 559, 12), // "target_title"
QT_MOC_LITERAL(44, 572, 4), // "mode"
QT_MOC_LITERAL(45, 577, 18), // "custom_instruction"
QT_MOC_LITERAL(46, 596, 17), // "fetch_url_content"
QT_MOC_LITERAL(47, 614, 15), // "read_local_file"
QT_MOC_LITERAL(48, 630, 9), // "file_path"
QT_MOC_LITERAL(49, 640, 14), // "confirm_action"
QT_MOC_LITERAL(50, 655, 8), // "approved"
QT_MOC_LITERAL(51, 664, 16), // "undo_last_action"
QT_MOC_LITERAL(52, 681, 11), // "poll_worker"
QT_MOC_LITERAL(53, 693, 12), // "fetch_models"
QT_MOC_LITERAL(54, 706, 11), // "poll_models"
QT_MOC_LITERAL(55, 718, 10), // "agent_busy"
QT_MOC_LITERAL(56, 729, 13), // "messages_json"
QT_MOC_LITERAL(57, 743, 19), // "pending_action_json"
QT_MOC_LITERAL(58, 763, 18), // "has_pending_action"
QT_MOC_LITERAL(59, 782, 8), // "can_undo"
QT_MOC_LITERAL(60, 791, 16), // "last_snapshot_id"
QT_MOC_LITERAL(61, 808, 17), // "last_created_note"
QT_MOC_LITERAL(62, 826, 13), // "error_message"
QT_MOC_LITERAL(63, 840, 14), // "streaming_text"
QT_MOC_LITERAL(64, 855, 11), // "is_fetching"
QT_MOC_LITERAL(65, 867, 13), // "provider_type"
QT_MOC_LITERAL(66, 881, 12), // "endpoint_url"
QT_MOC_LITERAL(67, 894, 10), // "model_name"
QT_MOC_LITERAL(68, 905, 12), // "timeout_secs"
QT_MOC_LITERAL(69, 918, 15), // "auto_allow_read"
QT_MOC_LITERAL(70, 934, 17), // "auto_allow_create"
QT_MOC_LITERAL(71, 952, 20), // "require_confirm_edit"
QT_MOC_LITERAL(72, 973, 15), // "allow_fetch_url"
QT_MOC_LITERAL(73, 989, 16), // "available_models"
QT_MOC_LITERAL(74, 1006, 14) // "models_loading"

    },
    "AgentBridge\0busy_changed\0\0messages_changed\0"
    "pending_action_changed\0undo_state_changed\0"
    "last_created_note_changed\0config_changed\0"
    "error_occurred\0message\0response_finished\0"
    "content\0undo_completed\0streaming_text_changed\0"
    "fetching_changed\0fetch_completed\0"
    "result\0fetch_error\0models_changed\0"
    "configure\0provider\0url\0model\0key\0"
    "timeout\0auto_read\0auto_create\0"
    "require_edit\0allow_self_signed\0"
    "allow_fetch\0reset_session\0context_filename\0"
    "context_content\0extra_context\0send_prompt\0"
    "text\0run_template\0template_id\0input_text\0"
    "run_custom_instruction\0instruction\0"
    "import_text\0source_text\0target_title\0"
    "mode\0custom_instruction\0fetch_url_content\0"
    "read_local_file\0file_path\0confirm_action\0"
    "approved\0undo_last_action\0poll_worker\0"
    "fetch_models\0poll_models\0agent_busy\0"
    "messages_json\0pending_action_json\0"
    "has_pending_action\0can_undo\0"
    "last_snapshot_id\0last_created_note\0"
    "error_message\0streaming_text\0is_fetching\0"
    "provider_type\0endpoint_url\0model_name\0"
    "timeout_secs\0auto_allow_read\0"
    "auto_allow_create\0require_confirm_edit\0"
    "allow_fetch_url\0available_models\0"
    "models_loading"
};
#undef QT_MOC_LITERAL

static const uint qt_meta_data_AgentBridge[] = {

 // content:
       7,       // revision
       0,       // classname
       0,    0, // classinfo
      27,   14, // methods
      21,  244, // properties
       0,    0, // enums/sets
       0,    0, // constructors
       0,       // flags
      14,       // signalCount

 // signals: name, argc, parameters, tag, flags
       1,    0,  149,    2, 0x06 /* Public */,
       3,    0,  150,    2, 0x06 /* Public */,
       4,    0,  151,    2, 0x06 /* Public */,
       5,    0,  152,    2, 0x06 /* Public */,
       6,    0,  153,    2, 0x06 /* Public */,
       7,    0,  154,    2, 0x06 /* Public */,
       8,    1,  155,    2, 0x06 /* Public */,
      10,    1,  158,    2, 0x06 /* Public */,
      12,    1,  161,    2, 0x06 /* Public */,
      13,    0,  164,    2, 0x06 /* Public */,
      14,    0,  165,    2, 0x06 /* Public */,
      15,    1,  166,    2, 0x06 /* Public */,
      17,    1,  169,    2, 0x06 /* Public */,
      18,    0,  172,    2, 0x06 /* Public */,

 // methods: name, argc, parameters, tag, flags
      19,   10,  173,    2, 0x02 /* Public */,
      30,    3,  194,    2, 0x02 /* Public */,
      34,    1,  201,    2, 0x02 /* Public */,
      36,    4,  204,    2, 0x02 /* Public */,
      39,    4,  213,    2, 0x02 /* Public */,
      41,    4,  222,    2, 0x02 /* Public */,
      46,    1,  231,    2, 0x02 /* Public */,
      47,    1,  234,    2, 0x02 /* Public */,
      49,    1,  237,    2, 0x02 /* Public */,
      51,    0,  240,    2, 0x02 /* Public */,
      52,    0,  241,    2, 0x02 /* Public */,
      53,    0,  242,    2, 0x02 /* Public */,
      54,    0,  243,    2, 0x02 /* Public */,

 // signals: parameters
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void, QMetaType::QString,    9,
    QMetaType::Void, QMetaType::QString,   11,
    QMetaType::Void, QMetaType::QString,    9,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void, QMetaType::QString,   16,
    QMetaType::Void, QMetaType::QString,    9,
    QMetaType::Void,

 // methods: parameters
    QMetaType::Void, QMetaType::QString, QMetaType::QString, QMetaType::QString, QMetaType::QString, QMetaType::Int, QMetaType::Bool, QMetaType::Bool, QMetaType::Bool, QMetaType::Bool, QMetaType::Bool,   20,   21,   22,   23,   24,   25,   26,   27,   28,   29,
    QMetaType::Void, QMetaType::QString, QMetaType::QString, QMetaType::QString,   31,   32,   33,
    QMetaType::Void, QMetaType::QString,   35,
    QMetaType::Void, QMetaType::QString, QMetaType::QString, QMetaType::QString, QMetaType::QString,   37,   38,   31,   32,
    QMetaType::Void, QMetaType::QString, QMetaType::QString, QMetaType::QString, QMetaType::QString,   40,   38,   31,   32,
    QMetaType::Void, QMetaType::QString, QMetaType::QString, QMetaType::QString, QMetaType::QString,   42,   43,   44,   45,
    QMetaType::Void, QMetaType::QString,   21,
    QMetaType::Void, QMetaType::QString,   48,
    QMetaType::Void, QMetaType::Bool,   50,
    QMetaType::Void,
    QMetaType::Bool,
    QMetaType::Void,
    QMetaType::Bool,

 // properties: name, type, flags
      55, QMetaType::Bool, 0x00495001,
      56, QMetaType::QString, 0x00495001,
      57, QMetaType::QString, 0x00495001,
      58, QMetaType::Bool, 0x00495001,
      59, QMetaType::Bool, 0x00495001,
      60, QMetaType::QString, 0x00495001,
      61, QMetaType::QString, 0x00495001,
      62, QMetaType::QString, 0x00495001,
      63, QMetaType::QString, 0x00495001,
      64, QMetaType::Bool, 0x00495001,
      65, QMetaType::QString, 0x00495001,
      66, QMetaType::QString, 0x00495001,
      67, QMetaType::QString, 0x00495001,
      68, QMetaType::Int, 0x00495001,
      69, QMetaType::Bool, 0x00495001,
      70, QMetaType::Bool, 0x00495001,
      71, QMetaType::Bool, 0x00495001,
      72, QMetaType::Bool, 0x00495001,
      28, QMetaType::Bool, 0x00495001,
      73, QMetaType::QString, 0x00495001,
      74, QMetaType::Bool, 0x00495001,

 // properties: notify_signal_id
       0,
       1,
       2,
       2,
       3,
       3,
       4,
       6,
       9,
      10,
       5,
       5,
       5,
       5,
       5,
       5,
       5,
       5,
       5,
      13,
      13,

       0        // eod
};

void AgentBridge::qt_static_metacall(QObject *_o, QMetaObject::Call _c, int _id, void **_a)
{
    if (_c == QMetaObject::InvokeMetaMethod) {
        AgentBridge *_t = static_cast<AgentBridge *>(_o);
        Q_UNUSED(_t)
        switch (_id) {
        case 0: _t->busy_changed(); break;
        case 1: _t->messages_changed(); break;
        case 2: _t->pending_action_changed(); break;
        case 3: _t->undo_state_changed(); break;
        case 4: _t->last_created_note_changed(); break;
        case 5: _t->config_changed(); break;
        case 6: _t->error_occurred((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 7: _t->response_finished((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 8: _t->undo_completed((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 9: _t->streaming_text_changed(); break;
        case 10: _t->fetching_changed(); break;
        case 11: _t->fetch_completed((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 12: _t->fetch_error((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 13: _t->models_changed(); break;
        case 14: _t->configure((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3])),(*reinterpret_cast< QString(*)>(_a[4])),(*reinterpret_cast< int(*)>(_a[5])),(*reinterpret_cast< bool(*)>(_a[6])),(*reinterpret_cast< bool(*)>(_a[7])),(*reinterpret_cast< bool(*)>(_a[8])),(*reinterpret_cast< bool(*)>(_a[9])),(*reinterpret_cast< bool(*)>(_a[10]))); break;
        case 15: _t->reset_session((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3]))); break;
        case 16: _t->send_prompt((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 17: _t->run_template((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3])),(*reinterpret_cast< QString(*)>(_a[4]))); break;
        case 18: _t->run_custom_instruction((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3])),(*reinterpret_cast< QString(*)>(_a[4]))); break;
        case 19: _t->import_text((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3])),(*reinterpret_cast< QString(*)>(_a[4]))); break;
        case 20: _t->fetch_url_content((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 21: _t->read_local_file((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 22: _t->confirm_action((*reinterpret_cast< bool(*)>(_a[1]))); break;
        case 23: _t->undo_last_action(); break;
        case 24: { bool _r = _t->poll_worker();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 25: _t->fetch_models(); break;
        case 26: { bool _r = _t->poll_models();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        default: ;
        }
    } else if (_c == QMetaObject::IndexOfMethod) {
        int *result = reinterpret_cast<int *>(_a[0]);
        void **func = reinterpret_cast<void **>(_a[1]);
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::busy_changed)) {
                *result = 0;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::messages_changed)) {
                *result = 1;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::pending_action_changed)) {
                *result = 2;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::undo_state_changed)) {
                *result = 3;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::last_created_note_changed)) {
                *result = 4;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::config_changed)) {
                *result = 5;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)(QString );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::error_occurred)) {
                *result = 6;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)(QString );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::response_finished)) {
                *result = 7;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)(QString );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::undo_completed)) {
                *result = 8;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::streaming_text_changed)) {
                *result = 9;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::fetching_changed)) {
                *result = 10;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)(QString );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::fetch_completed)) {
                *result = 11;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)(QString );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::fetch_error)) {
                *result = 12;
                return;
            }
        }
        {
            typedef void (AgentBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&AgentBridge::models_changed)) {
                *result = 13;
                return;
            }
        }
    }
#ifndef QT_NO_PROPERTIES
    else if (_c == QMetaObject::ReadProperty) {
        AgentBridge *_t = static_cast<AgentBridge *>(_o);
        Q_UNUSED(_t)
        void *_v = _a[0];
        switch (_id) {
        case 0: *reinterpret_cast< bool*>(_v) = _t->agentBusy(); break;
        case 1: *reinterpret_cast< QString*>(_v) = _t->messagesJson(); break;
        case 2: *reinterpret_cast< QString*>(_v) = _t->pendingActionJson(); break;
        case 3: *reinterpret_cast< bool*>(_v) = _t->hasPendingAction(); break;
        case 4: *reinterpret_cast< bool*>(_v) = _t->canUndo(); break;
        case 5: *reinterpret_cast< QString*>(_v) = _t->lastSnapshotId(); break;
        case 6: *reinterpret_cast< QString*>(_v) = _t->lastCreatedNote(); break;
        case 7: *reinterpret_cast< QString*>(_v) = _t->errorMessage(); break;
        case 8: *reinterpret_cast< QString*>(_v) = _t->streamingText(); break;
        case 9: *reinterpret_cast< bool*>(_v) = _t->isFetching(); break;
        case 10: *reinterpret_cast< QString*>(_v) = _t->providerType(); break;
        case 11: *reinterpret_cast< QString*>(_v) = _t->endpointUrl(); break;
        case 12: *reinterpret_cast< QString*>(_v) = _t->modelName(); break;
        case 13: *reinterpret_cast< int*>(_v) = _t->timeoutSecs(); break;
        case 14: *reinterpret_cast< bool*>(_v) = _t->autoAllowRead(); break;
        case 15: *reinterpret_cast< bool*>(_v) = _t->autoAllowCreate(); break;
        case 16: *reinterpret_cast< bool*>(_v) = _t->requireConfirmEdit(); break;
        case 17: *reinterpret_cast< bool*>(_v) = _t->allowFetchUrl(); break;
        case 18: *reinterpret_cast< bool*>(_v) = _t->allowSelfSigned(); break;
        case 19: *reinterpret_cast< QString*>(_v) = _t->availableModels(); break;
        case 20: *reinterpret_cast< bool*>(_v) = _t->modelsLoading(); break;
        default: break;
        }
    } else if (_c == QMetaObject::WriteProperty) {
    } else if (_c == QMetaObject::ResetProperty) {
    }
#endif // QT_NO_PROPERTIES
}

const QMetaObject AgentBridge::staticMetaObject = {
    { &QObject::staticMetaObject, qt_meta_stringdata_AgentBridge.data,
      qt_meta_data_AgentBridge,  qt_static_metacall, Q_NULLPTR, Q_NULLPTR}
};


const QMetaObject *AgentBridge::metaObject() const
{
    return QObject::d_ptr->metaObject ? QObject::d_ptr->dynamicMetaObject() : &staticMetaObject;
}

void *AgentBridge::qt_metacast(const char *_clname)
{
    if (!_clname) return Q_NULLPTR;
    if (!strcmp(_clname, qt_meta_stringdata_AgentBridge.stringdata0))
        return static_cast<void*>(const_cast< AgentBridge*>(this));
    return QObject::qt_metacast(_clname);
}

int AgentBridge::qt_metacall(QMetaObject::Call _c, int _id, void **_a)
{
    _id = QObject::qt_metacall(_c, _id, _a);
    if (_id < 0)
        return _id;
    if (_c == QMetaObject::InvokeMetaMethod) {
        if (_id < 27)
            qt_static_metacall(this, _c, _id, _a);
        _id -= 27;
    } else if (_c == QMetaObject::RegisterMethodArgumentMetaType) {
        if (_id < 27)
            *reinterpret_cast<int*>(_a[0]) = -1;
        _id -= 27;
    }
#ifndef QT_NO_PROPERTIES
   else if (_c == QMetaObject::ReadProperty || _c == QMetaObject::WriteProperty
            || _c == QMetaObject::ResetProperty || _c == QMetaObject::RegisterPropertyMetaType) {
        qt_static_metacall(this, _c, _id, _a);
        _id -= 21;
    } else if (_c == QMetaObject::QueryPropertyDesignable) {
        _id -= 21;
    } else if (_c == QMetaObject::QueryPropertyScriptable) {
        _id -= 21;
    } else if (_c == QMetaObject::QueryPropertyStored) {
        _id -= 21;
    } else if (_c == QMetaObject::QueryPropertyEditable) {
        _id -= 21;
    } else if (_c == QMetaObject::QueryPropertyUser) {
        _id -= 21;
    }
#endif // QT_NO_PROPERTIES
    return _id;
}

// SIGNAL 0
void AgentBridge::busy_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 0, Q_NULLPTR);
}

// SIGNAL 1
void AgentBridge::messages_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 1, Q_NULLPTR);
}

// SIGNAL 2
void AgentBridge::pending_action_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 2, Q_NULLPTR);
}

// SIGNAL 3
void AgentBridge::undo_state_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 3, Q_NULLPTR);
}

// SIGNAL 4
void AgentBridge::last_created_note_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 4, Q_NULLPTR);
}

// SIGNAL 5
void AgentBridge::config_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 5, Q_NULLPTR);
}

// SIGNAL 6
void AgentBridge::error_occurred(QString _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 6, _a);
}

// SIGNAL 7
void AgentBridge::response_finished(QString _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 7, _a);
}

// SIGNAL 8
void AgentBridge::undo_completed(QString _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 8, _a);
}

// SIGNAL 9
void AgentBridge::streaming_text_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 9, Q_NULLPTR);
}

// SIGNAL 10
void AgentBridge::fetching_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 10, Q_NULLPTR);
}

// SIGNAL 11
void AgentBridge::fetch_completed(QString _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 11, _a);
}

// SIGNAL 12
void AgentBridge::fetch_error(QString _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 12, _a);
}

// SIGNAL 13
void AgentBridge::models_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 13, Q_NULLPTR);
}
QT_END_MOC_NAMESPACE
