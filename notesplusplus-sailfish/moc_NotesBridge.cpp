/****************************************************************************
** Meta object code from reading C++ file 'NotesBridge.h'
**
** Created by: The Qt Meta Object Compiler version 67 (Qt 5.6.3)
**
** WARNING! All changes made in this file will be lost!
*****************************************************************************/

#include "src/bridge/NotesBridge.h"
#include <QtCore/qbytearray.h>
#include <QtCore/qmetatype.h>
#if !defined(Q_MOC_OUTPUT_REVISION)
#error "The header file 'NotesBridge.h' doesn't include <QObject>."
#elif Q_MOC_OUTPUT_REVISION != 67
#error "This file was generated using the moc from 5.6.3. It"
#error "cannot be used with the include files from this version of Qt."
#error "(The moc has changed too much.)"
#endif

QT_BEGIN_MOC_NAMESPACE
struct qt_meta_stringdata_NotesBridge_t {
    QByteArrayData data[141];
    char stringdata0[2170];
};
#define QT_MOC_LITERAL(idx, ofs, len) \
    Q_STATIC_BYTE_ARRAY_DATA_HEADER_INITIALIZER_WITH_OFFSET(len, \
    qptrdiff(offsetof(qt_meta_stringdata_NotesBridge_t, stringdata0) + ofs \
        - idx * sizeof(QByteArrayData)) \
    )
static const qt_meta_stringdata_NotesBridge_t qt_meta_stringdata_NotesBridge = {
    {
QT_MOC_LITERAL(0, 0, 11), // "NotesBridge"
QT_MOC_LITERAL(1, 12, 12), // "page_changed"
QT_MOC_LITERAL(2, 25, 0), // ""
QT_MOC_LITERAL(3, 26, 31), // "current_page_group_path_changed"
QT_MOC_LITERAL(4, 58, 30), // "current_page_full_path_changed"
QT_MOC_LITERAL(5, 89, 22), // "search_results_changed"
QT_MOC_LITERAL(6, 112, 14), // "data_refreshed"
QT_MOC_LITERAL(7, 127, 19), // "group_depth_changed"
QT_MOC_LITERAL(8, 147, 15), // "loading_changed"
QT_MOC_LITERAL(9, 163, 21), // "drop_comments_changed"
QT_MOC_LITERAL(10, 185, 30), // "reject_public_networks_changed"
QT_MOC_LITERAL(11, 216, 20), // "bind_address_changed"
QT_MOC_LITERAL(12, 237, 25), // "web_server_status_changed"
QT_MOC_LITERAL(13, 263, 14), // "error_occurred"
QT_MOC_LITERAL(14, 278, 7), // "message"
QT_MOC_LITERAL(15, 286, 19), // "initialized_changed"
QT_MOC_LITERAL(16, 306, 22), // "auth_challenge_changed"
QT_MOC_LITERAL(17, 329, 9), // "load_page"
QT_MOC_LITERAL(18, 339, 4), // "name"
QT_MOC_LITERAL(19, 344, 10), // "save_block"
QT_MOC_LITERAL(20, 355, 5), // "index"
QT_MOC_LITERAL(21, 361, 8), // "raw_text"
QT_MOC_LITERAL(22, 370, 16), // "save_block_range"
QT_MOC_LITERAL(23, 387, 11), // "start_index"
QT_MOC_LITERAL(24, 399, 5), // "count"
QT_MOC_LITERAL(25, 405, 22), // "append_to_current_page"
QT_MOC_LITERAL(26, 428, 4), // "text"
QT_MOC_LITERAL(27, 433, 7), // "is_task"
QT_MOC_LITERAL(28, 441, 18), // "save_journal_block"
QT_MOC_LITERAL(29, 460, 23), // "toggle_journal_checkbox"
QT_MOC_LITERAL(30, 484, 11), // "block_index"
QT_MOC_LITERAL(31, 496, 9), // "item_path"
QT_MOC_LITERAL(32, 506, 17), // "append_to_journal"
QT_MOC_LITERAL(33, 524, 15), // "get_page_source"
QT_MOC_LITERAL(34, 540, 16), // "save_page_source"
QT_MOC_LITERAL(35, 557, 7), // "content"
QT_MOC_LITERAL(36, 565, 11), // "create_page"
QT_MOC_LITERAL(37, 577, 11), // "delete_page"
QT_MOC_LITERAL(38, 589, 16), // "navigate_to_page"
QT_MOC_LITERAL(39, 606, 21), // "insert_link_at_cursor"
QT_MOC_LITERAL(40, 628, 9), // "block_idx"
QT_MOC_LITERAL(41, 638, 10), // "cursor_pos"
QT_MOC_LITERAL(42, 649, 6), // "target"
QT_MOC_LITERAL(43, 656, 15), // "toggle_checkbox"
QT_MOC_LITERAL(44, 672, 12), // "create_group"
QT_MOC_LITERAL(45, 685, 11), // "parent_path"
QT_MOC_LITERAL(46, 697, 12), // "rename_group"
QT_MOC_LITERAL(47, 710, 8), // "old_path"
QT_MOC_LITERAL(48, 719, 8), // "new_name"
QT_MOC_LITERAL(49, 728, 12), // "delete_group"
QT_MOC_LITERAL(50, 741, 4), // "path"
QT_MOC_LITERAL(51, 746, 9), // "recursive"
QT_MOC_LITERAL(52, 756, 18), // "move_page_to_group"
QT_MOC_LITERAL(53, 775, 14), // "page_full_path"
QT_MOC_LITERAL(54, 790, 12), // "target_group"
QT_MOC_LITERAL(55, 803, 23), // "set_group_display_depth"
QT_MOC_LITERAL(56, 827, 5), // "depth"
QT_MOC_LITERAL(57, 833, 22), // "toggle_group_collapsed"
QT_MOC_LITERAL(58, 856, 10), // "group_path"
QT_MOC_LITERAL(59, 867, 19), // "set_group_note_sort"
QT_MOC_LITERAL(60, 887, 9), // "note_sort"
QT_MOC_LITERAL(61, 897, 19), // "get_group_note_sort"
QT_MOC_LITERAL(62, 917, 15), // "get_groups_json"
QT_MOC_LITERAL(63, 933, 13), // "rebuild_index"
QT_MOC_LITERAL(64, 947, 9), // "do_search"
QT_MOC_LITERAL(65, 957, 5), // "query"
QT_MOC_LITERAL(66, 963, 6), // "search"
QT_MOC_LITERAL(67, 970, 11), // "poll_search"
QT_MOC_LITERAL(68, 982, 20), // "poll_search_previews"
QT_MOC_LITERAL(69, 1003, 23), // "get_linkable_pages_json"
QT_MOC_LITERAL(70, 1027, 19), // "load_main_page_data"
QT_MOC_LITERAL(71, 1047, 19), // "poll_main_page_data"
QT_MOC_LITERAL(72, 1067, 12), // "poll_results"
QT_MOC_LITERAL(73, 1080, 11), // "export_html"
QT_MOC_LITERAL(74, 1092, 9), // "page_name"
QT_MOC_LITERAL(75, 1102, 15), // "export_all_html"
QT_MOC_LITERAL(76, 1118, 15), // "open_in_browser"
QT_MOC_LITERAL(77, 1134, 20), // "get_server_urls_json"
QT_MOC_LITERAL(78, 1155, 16), // "start_web_server"
QT_MOC_LITERAL(79, 1172, 15), // "stop_web_server"
QT_MOC_LITERAL(80, 1188, 17), // "toggle_web_server"
QT_MOC_LITERAL(81, 1206, 12), // "configure_ai"
QT_MOC_LITERAL(82, 1219, 8), // "provider"
QT_MOC_LITERAL(83, 1228, 3), // "url"
QT_MOC_LITERAL(84, 1232, 5), // "model"
QT_MOC_LITERAL(85, 1238, 3), // "key"
QT_MOC_LITERAL(86, 1242, 7), // "timeout"
QT_MOC_LITERAL(87, 1250, 9), // "auto_read"
QT_MOC_LITERAL(88, 1260, 11), // "auto_create"
QT_MOC_LITERAL(89, 1272, 12), // "require_edit"
QT_MOC_LITERAL(90, 1285, 17), // "allow_self_signed"
QT_MOC_LITERAL(91, 1303, 11), // "allow_fetch"
QT_MOC_LITERAL(92, 1315, 23), // "install_tls_certificate"
QT_MOC_LITERAL(93, 1339, 16), // "cert_pem_or_path"
QT_MOC_LITERAL(94, 1356, 15), // "key_pem_or_path"
QT_MOC_LITERAL(95, 1372, 21), // "reset_tls_certificate"
QT_MOC_LITERAL(96, 1394, 25), // "is_custom_tls_certificate"
QT_MOC_LITERAL(97, 1420, 29), // "get_tls_certificate_info_json"
QT_MOC_LITERAL(98, 1450, 17), // "set_drop_comments"
QT_MOC_LITERAL(99, 1468, 4), // "drop"
QT_MOC_LITERAL(100, 1473, 26), // "set_reject_public_networks"
QT_MOC_LITERAL(101, 1500, 6), // "reject"
QT_MOC_LITERAL(102, 1507, 16), // "set_bind_address"
QT_MOC_LITERAL(103, 1524, 4), // "addr"
QT_MOC_LITERAL(104, 1529, 27), // "get_network_interfaces_json"
QT_MOC_LITERAL(105, 1557, 9), // "set_theme"
QT_MOC_LITERAL(106, 1567, 11), // "colors_json"
QT_MOC_LITERAL(107, 1579, 24), // "set_session_expiry_hours"
QT_MOC_LITERAL(108, 1604, 5), // "hours"
QT_MOC_LITERAL(109, 1610, 20), // "check_auth_challenge"
QT_MOC_LITERAL(110, 1631, 22), // "approve_auth_challenge"
QT_MOC_LITERAL(111, 1654, 12), // "challenge_id"
QT_MOC_LITERAL(112, 1667, 19), // "deny_auth_challenge"
QT_MOC_LITERAL(113, 1687, 23), // "render_element_previews"
QT_MOC_LITERAL(114, 1711, 17), // "current_page_name"
QT_MOC_LITERAL(115, 1729, 23), // "current_page_group_path"
QT_MOC_LITERAL(116, 1753, 22), // "current_page_full_path"
QT_MOC_LITERAL(117, 1776, 22), // "current_page_file_path"
QT_MOC_LITERAL(118, 1799, 14), // "current_blocks"
QT_MOC_LITERAL(119, 1814, 15), // "is_journal_page"
QT_MOC_LITERAL(120, 1830, 14), // "blocks_version"
QT_MOC_LITERAL(121, 1845, 9), // "notes_dir"
QT_MOC_LITERAL(122, 1855, 12), // "search_query"
QT_MOC_LITERAL(123, 1868, 14), // "search_results"
QT_MOC_LITERAL(124, 1883, 14), // "search_loading"
QT_MOC_LITERAL(125, 1898, 12), // "recent_pages"
QT_MOC_LITERAL(126, 1911, 17), // "grouped_tree_json"
QT_MOC_LITERAL(127, 1929, 19), // "group_display_depth"
QT_MOC_LITERAL(128, 1949, 20), // "recent_journal_lines"
QT_MOC_LITERAL(129, 1970, 14), // "journal_blocks"
QT_MOC_LITERAL(130, 1985, 10), // "is_loading"
QT_MOC_LITERAL(131, 1996, 13), // "drop_comments"
QT_MOC_LITERAL(132, 2010, 22), // "reject_public_networks"
QT_MOC_LITERAL(133, 2033, 12), // "bind_address"
QT_MOC_LITERAL(134, 2046, 18), // "web_server_running"
QT_MOC_LITERAL(135, 2065, 14), // "web_server_url"
QT_MOC_LITERAL(136, 2080, 13), // "error_message"
QT_MOC_LITERAL(137, 2094, 11), // "initialized"
QT_MOC_LITERAL(138, 2106, 22), // "auth_challenge_pending"
QT_MOC_LITERAL(139, 2129, 17), // "auth_challenge_id"
QT_MOC_LITERAL(140, 2147, 22) // "auth_verification_code"

    },
    "NotesBridge\0page_changed\0\0"
    "current_page_group_path_changed\0"
    "current_page_full_path_changed\0"
    "search_results_changed\0data_refreshed\0"
    "group_depth_changed\0loading_changed\0"
    "drop_comments_changed\0"
    "reject_public_networks_changed\0"
    "bind_address_changed\0web_server_status_changed\0"
    "error_occurred\0message\0initialized_changed\0"
    "auth_challenge_changed\0load_page\0name\0"
    "save_block\0index\0raw_text\0save_block_range\0"
    "start_index\0count\0append_to_current_page\0"
    "text\0is_task\0save_journal_block\0"
    "toggle_journal_checkbox\0block_index\0"
    "item_path\0append_to_journal\0get_page_source\0"
    "save_page_source\0content\0create_page\0"
    "delete_page\0navigate_to_page\0"
    "insert_link_at_cursor\0block_idx\0"
    "cursor_pos\0target\0toggle_checkbox\0"
    "create_group\0parent_path\0rename_group\0"
    "old_path\0new_name\0delete_group\0path\0"
    "recursive\0move_page_to_group\0"
    "page_full_path\0target_group\0"
    "set_group_display_depth\0depth\0"
    "toggle_group_collapsed\0group_path\0"
    "set_group_note_sort\0note_sort\0"
    "get_group_note_sort\0get_groups_json\0"
    "rebuild_index\0do_search\0query\0search\0"
    "poll_search\0poll_search_previews\0"
    "get_linkable_pages_json\0load_main_page_data\0"
    "poll_main_page_data\0poll_results\0"
    "export_html\0page_name\0export_all_html\0"
    "open_in_browser\0get_server_urls_json\0"
    "start_web_server\0stop_web_server\0"
    "toggle_web_server\0configure_ai\0provider\0"
    "url\0model\0key\0timeout\0auto_read\0"
    "auto_create\0require_edit\0allow_self_signed\0"
    "allow_fetch\0install_tls_certificate\0"
    "cert_pem_or_path\0key_pem_or_path\0"
    "reset_tls_certificate\0is_custom_tls_certificate\0"
    "get_tls_certificate_info_json\0"
    "set_drop_comments\0drop\0"
    "set_reject_public_networks\0reject\0"
    "set_bind_address\0addr\0get_network_interfaces_json\0"
    "set_theme\0colors_json\0set_session_expiry_hours\0"
    "hours\0check_auth_challenge\0"
    "approve_auth_challenge\0challenge_id\0"
    "deny_auth_challenge\0render_element_previews\0"
    "current_page_name\0current_page_group_path\0"
    "current_page_full_path\0current_page_file_path\0"
    "current_blocks\0is_journal_page\0"
    "blocks_version\0notes_dir\0search_query\0"
    "search_results\0search_loading\0"
    "recent_pages\0grouped_tree_json\0"
    "group_display_depth\0recent_journal_lines\0"
    "journal_blocks\0is_loading\0drop_comments\0"
    "reject_public_networks\0bind_address\0"
    "web_server_running\0web_server_url\0"
    "error_message\0initialized\0"
    "auth_challenge_pending\0auth_challenge_id\0"
    "auth_verification_code"
};
#undef QT_MOC_LITERAL

static const uint qt_meta_data_NotesBridge[] = {

 // content:
       7,       // revision
       0,       // classname
       0,    0, // classinfo
      68,   14, // methods
      27,  548, // properties
       0,    0, // enums/sets
       0,    0, // constructors
       0,       // flags
      14,       // signalCount

 // signals: name, argc, parameters, tag, flags
       1,    0,  354,    2, 0x06 /* Public */,
       3,    0,  355,    2, 0x06 /* Public */,
       4,    0,  356,    2, 0x06 /* Public */,
       5,    0,  357,    2, 0x06 /* Public */,
       6,    0,  358,    2, 0x06 /* Public */,
       7,    0,  359,    2, 0x06 /* Public */,
       8,    0,  360,    2, 0x06 /* Public */,
       9,    0,  361,    2, 0x06 /* Public */,
      10,    0,  362,    2, 0x06 /* Public */,
      11,    0,  363,    2, 0x06 /* Public */,
      12,    0,  364,    2, 0x06 /* Public */,
      13,    1,  365,    2, 0x06 /* Public */,
      15,    0,  368,    2, 0x06 /* Public */,
      16,    0,  369,    2, 0x06 /* Public */,

 // methods: name, argc, parameters, tag, flags
      17,    1,  370,    2, 0x02 /* Public */,
      19,    2,  373,    2, 0x02 /* Public */,
      22,    3,  378,    2, 0x02 /* Public */,
      25,    2,  385,    2, 0x02 /* Public */,
      28,    2,  390,    2, 0x02 /* Public */,
      29,    2,  395,    2, 0x02 /* Public */,
      32,    2,  400,    2, 0x02 /* Public */,
      33,    1,  405,    2, 0x02 /* Public */,
      34,    2,  408,    2, 0x02 /* Public */,
      36,    1,  413,    2, 0x02 /* Public */,
      37,    1,  416,    2, 0x02 /* Public */,
      38,    1,  419,    2, 0x02 /* Public */,
      39,    3,  422,    2, 0x02 /* Public */,
      43,    2,  429,    2, 0x02 /* Public */,
      44,    2,  434,    2, 0x02 /* Public */,
      46,    2,  439,    2, 0x02 /* Public */,
      49,    2,  444,    2, 0x02 /* Public */,
      52,    2,  449,    2, 0x02 /* Public */,
      55,    1,  454,    2, 0x02 /* Public */,
      57,    1,  457,    2, 0x02 /* Public */,
      59,    2,  460,    2, 0x02 /* Public */,
      61,    1,  465,    2, 0x02 /* Public */,
      62,    0,  468,    2, 0x02 /* Public */,
      63,    0,  469,    2, 0x02 /* Public */,
      64,    1,  470,    2, 0x02 /* Public */,
      66,    1,  473,    2, 0x02 /* Public */,
      67,    0,  476,    2, 0x02 /* Public */,
      68,    0,  477,    2, 0x02 /* Public */,
      69,    1,  478,    2, 0x02 /* Public */,
      70,    0,  481,    2, 0x02 /* Public */,
      71,    0,  482,    2, 0x02 /* Public */,
      72,    0,  483,    2, 0x02 /* Public */,
      73,    1,  484,    2, 0x02 /* Public */,
      75,    0,  487,    2, 0x02 /* Public */,
      76,    1,  488,    2, 0x02 /* Public */,
      77,    0,  491,    2, 0x02 /* Public */,
      78,    0,  492,    2, 0x02 /* Public */,
      79,    0,  493,    2, 0x02 /* Public */,
      80,    0,  494,    2, 0x02 /* Public */,
      81,   10,  495,    2, 0x02 /* Public */,
      92,    2,  516,    2, 0x02 /* Public */,
      95,    0,  521,    2, 0x02 /* Public */,
      96,    0,  522,    2, 0x02 /* Public */,
      97,    0,  523,    2, 0x02 /* Public */,
      98,    1,  524,    2, 0x02 /* Public */,
     100,    1,  527,    2, 0x02 /* Public */,
     102,    1,  530,    2, 0x02 /* Public */,
     104,    0,  533,    2, 0x02 /* Public */,
     105,    1,  534,    2, 0x02 /* Public */,
     107,    1,  537,    2, 0x02 /* Public */,
     109,    0,  540,    2, 0x02 /* Public */,
     110,    1,  541,    2, 0x02 /* Public */,
     112,    1,  544,    2, 0x02 /* Public */,
     113,    0,  547,    2, 0x02 /* Public */,

 // signals: parameters
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void,
    QMetaType::Void, QMetaType::QString,   14,
    QMetaType::Void,
    QMetaType::Void,

 // methods: parameters
    QMetaType::Void, QMetaType::QString,   18,
    QMetaType::Void, QMetaType::Int, QMetaType::QString,   20,   21,
    QMetaType::Void, QMetaType::Int, QMetaType::Int, QMetaType::QString,   23,   24,   21,
    QMetaType::Void, QMetaType::QString, QMetaType::Bool,   26,   27,
    QMetaType::Void, QMetaType::Int, QMetaType::QString,   20,   21,
    QMetaType::Void, QMetaType::Int, QMetaType::QString,   30,   31,
    QMetaType::Void, QMetaType::QString, QMetaType::Bool,   26,   27,
    QMetaType::QString, QMetaType::QString,   18,
    QMetaType::Void, QMetaType::QString, QMetaType::QString,   18,   35,
    QMetaType::Void, QMetaType::QString,   18,
    QMetaType::Void, QMetaType::QString,   18,
    QMetaType::Void, QMetaType::QString,   18,
    QMetaType::Void, QMetaType::Int, QMetaType::Int, QMetaType::QString,   40,   41,   42,
    QMetaType::Void, QMetaType::Int, QMetaType::QString,   30,   31,
    QMetaType::Bool, QMetaType::QString, QMetaType::QString,   45,   18,
    QMetaType::Bool, QMetaType::QString, QMetaType::QString,   47,   48,
    QMetaType::Bool, QMetaType::QString, QMetaType::Bool,   50,   51,
    QMetaType::Bool, QMetaType::QString, QMetaType::QString,   53,   54,
    QMetaType::Void, QMetaType::Int,   56,
    QMetaType::Bool, QMetaType::QString,   58,
    QMetaType::Bool, QMetaType::QString, QMetaType::QString,   58,   60,
    QMetaType::QString, QMetaType::QString,   58,
    QMetaType::QString,
    QMetaType::QString,
    QMetaType::Void, QMetaType::QString,   65,
    QMetaType::Void, QMetaType::QString,   65,
    QMetaType::Bool,
    QMetaType::Bool,
    QMetaType::QString, QMetaType::QString,   65,
    QMetaType::Void,
    QMetaType::Bool,
    QMetaType::Bool,
    QMetaType::QString, QMetaType::QString,   74,
    QMetaType::QString,
    QMetaType::QString, QMetaType::QString,   74,
    QMetaType::QString,
    QMetaType::QString,
    QMetaType::Void,
    QMetaType::Bool,
    QMetaType::Void, QMetaType::QString, QMetaType::QString, QMetaType::QString, QMetaType::QString, QMetaType::Int, QMetaType::Bool, QMetaType::Bool, QMetaType::Bool, QMetaType::Bool, QMetaType::Bool,   82,   83,   84,   85,   86,   87,   88,   89,   90,   91,
    QMetaType::QString, QMetaType::QString, QMetaType::QString,   93,   94,
    QMetaType::QString,
    QMetaType::Bool,
    QMetaType::QString,
    QMetaType::Void, QMetaType::Bool,   99,
    QMetaType::Void, QMetaType::Bool,  101,
    QMetaType::Void, QMetaType::QString,  103,
    QMetaType::QString,
    QMetaType::Void, QMetaType::QString,  106,
    QMetaType::Void, QMetaType::Int,  108,
    QMetaType::Bool,
    QMetaType::Void, QMetaType::QString,  111,
    QMetaType::Void, QMetaType::QString,  111,
    QMetaType::QString,

 // properties: name, type, flags
     114, QMetaType::QString, 0x00495003,
     115, QMetaType::QString, 0x00495003,
     116, QMetaType::QString, 0x00495003,
     117, QMetaType::QString, 0x00495003,
     118, QMetaType::QVariantList, 0x00495003,
     119, QMetaType::Bool, 0x00495003,
     120, QMetaType::Int, 0x00495003,
     121, QMetaType::QString, 0x00495003,
     122, QMetaType::QString, 0x00495003,
     123, QMetaType::QVariantList, 0x00495003,
     124, QMetaType::Bool, 0x00495003,
     125, QMetaType::QVariantList, 0x00495003,
     126, QMetaType::QString, 0x00495003,
     127, QMetaType::Int, 0x00495003,
     128, QMetaType::QVariantList, 0x00495003,
     129, QMetaType::QVariantList, 0x00495003,
     130, QMetaType::Bool, 0x00495003,
     131, QMetaType::Bool, 0x00495003,
     132, QMetaType::Bool, 0x00495003,
     133, QMetaType::QString, 0x00495003,
     134, QMetaType::Bool, 0x00495003,
     135, QMetaType::QString, 0x00495003,
     136, QMetaType::QString, 0x00495003,
     137, QMetaType::Bool, 0x00495003,
     138, QMetaType::Bool, 0x00495003,
     139, QMetaType::QString, 0x00495003,
     140, QMetaType::QString, 0x00495003,

 // properties: notify_signal_id
       0,
       1,
       2,
       0,
       0,
       0,
       0,
       0,
       3,
       3,
       6,
       4,
       4,
       5,
       4,
       4,
       6,
       7,
       8,
       9,
      10,
      10,
      11,
      12,
      13,
      13,
      13,

       0        // eod
};

void NotesBridge::qt_static_metacall(QObject *_o, QMetaObject::Call _c, int _id, void **_a)
{
    if (_c == QMetaObject::InvokeMetaMethod) {
        NotesBridge *_t = static_cast<NotesBridge *>(_o);
        Q_UNUSED(_t)
        switch (_id) {
        case 0: _t->page_changed(); break;
        case 1: _t->current_page_group_path_changed(); break;
        case 2: _t->current_page_full_path_changed(); break;
        case 3: _t->search_results_changed(); break;
        case 4: _t->data_refreshed(); break;
        case 5: _t->group_depth_changed(); break;
        case 6: _t->loading_changed(); break;
        case 7: _t->drop_comments_changed(); break;
        case 8: _t->reject_public_networks_changed(); break;
        case 9: _t->bind_address_changed(); break;
        case 10: _t->web_server_status_changed(); break;
        case 11: _t->error_occurred((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 12: _t->initialized_changed(); break;
        case 13: _t->auth_challenge_changed(); break;
        case 14: _t->load_page((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 15: _t->save_block((*reinterpret_cast< int(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2]))); break;
        case 16: _t->save_block_range((*reinterpret_cast< int(*)>(_a[1])),(*reinterpret_cast< int(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3]))); break;
        case 17: _t->append_to_current_page((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< bool(*)>(_a[2]))); break;
        case 18: _t->save_journal_block((*reinterpret_cast< int(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2]))); break;
        case 19: _t->toggle_journal_checkbox((*reinterpret_cast< int(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2]))); break;
        case 20: _t->append_to_journal((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< bool(*)>(_a[2]))); break;
        case 21: { QString _r = _t->get_page_source((*reinterpret_cast< QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 22: _t->save_page_source((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2]))); break;
        case 23: _t->create_page((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 24: _t->delete_page((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 25: _t->navigate_to_page((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 26: _t->insert_link_at_cursor((*reinterpret_cast< int(*)>(_a[1])),(*reinterpret_cast< int(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3]))); break;
        case 27: _t->toggle_checkbox((*reinterpret_cast< int(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2]))); break;
        case 28: { bool _r = _t->create_group((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 29: { bool _r = _t->rename_group((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 30: { bool _r = _t->delete_group((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< bool(*)>(_a[2])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 31: { bool _r = _t->move_page_to_group((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 32: _t->set_group_display_depth((*reinterpret_cast< int(*)>(_a[1]))); break;
        case 33: { bool _r = _t->toggle_group_collapsed((*reinterpret_cast< QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 34: { bool _r = _t->set_group_note_sort((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])));
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 35: { QString _r = _t->get_group_note_sort((*reinterpret_cast< QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 36: { QString _r = _t->get_groups_json();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 37: { QString _r = _t->rebuild_index();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 38: _t->do_search((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 39: _t->search((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 40: { bool _r = _t->poll_search();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 41: { bool _r = _t->poll_search_previews();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 42: { QString _r = _t->get_linkable_pages_json((*reinterpret_cast< QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 43: _t->load_main_page_data(); break;
        case 44: { bool _r = _t->poll_main_page_data();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 45: { bool _r = _t->poll_results();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 46: { QString _r = _t->export_html((*reinterpret_cast< QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 47: { QString _r = _t->export_all_html();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 48: { QString _r = _t->open_in_browser((*reinterpret_cast< QString(*)>(_a[1])));
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 49: { QString _r = _t->get_server_urls_json();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 50: { QString _r = _t->start_web_server();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 51: _t->stop_web_server(); break;
        case 52: { bool _r = _t->toggle_web_server();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 53: _t->configure_ai((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])),(*reinterpret_cast< QString(*)>(_a[3])),(*reinterpret_cast< QString(*)>(_a[4])),(*reinterpret_cast< int(*)>(_a[5])),(*reinterpret_cast< bool(*)>(_a[6])),(*reinterpret_cast< bool(*)>(_a[7])),(*reinterpret_cast< bool(*)>(_a[8])),(*reinterpret_cast< bool(*)>(_a[9])),(*reinterpret_cast< bool(*)>(_a[10]))); break;
        case 54: { QString _r = _t->install_tls_certificate((*reinterpret_cast< QString(*)>(_a[1])),(*reinterpret_cast< QString(*)>(_a[2])));
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 55: { QString _r = _t->reset_tls_certificate();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 56: { bool _r = _t->is_custom_tls_certificate();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 57: { QString _r = _t->get_tls_certificate_info_json();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 58: _t->set_drop_comments((*reinterpret_cast< bool(*)>(_a[1]))); break;
        case 59: _t->set_reject_public_networks((*reinterpret_cast< bool(*)>(_a[1]))); break;
        case 60: _t->set_bind_address((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 61: { QString _r = _t->get_network_interfaces_json();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        case 62: _t->set_theme((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 63: _t->set_session_expiry_hours((*reinterpret_cast< int(*)>(_a[1]))); break;
        case 64: { bool _r = _t->check_auth_challenge();
            if (_a[0]) *reinterpret_cast< bool*>(_a[0]) = _r; }  break;
        case 65: _t->approve_auth_challenge((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 66: _t->deny_auth_challenge((*reinterpret_cast< QString(*)>(_a[1]))); break;
        case 67: { QString _r = _t->render_element_previews();
            if (_a[0]) *reinterpret_cast< QString*>(_a[0]) = _r; }  break;
        default: ;
        }
    } else if (_c == QMetaObject::IndexOfMethod) {
        int *result = reinterpret_cast<int *>(_a[0]);
        void **func = reinterpret_cast<void **>(_a[1]);
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::page_changed)) {
                *result = 0;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::current_page_group_path_changed)) {
                *result = 1;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::current_page_full_path_changed)) {
                *result = 2;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::search_results_changed)) {
                *result = 3;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::data_refreshed)) {
                *result = 4;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::group_depth_changed)) {
                *result = 5;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::loading_changed)) {
                *result = 6;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::drop_comments_changed)) {
                *result = 7;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::reject_public_networks_changed)) {
                *result = 8;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::bind_address_changed)) {
                *result = 9;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::web_server_status_changed)) {
                *result = 10;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)(QString );
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::error_occurred)) {
                *result = 11;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::initialized_changed)) {
                *result = 12;
                return;
            }
        }
        {
            typedef void (NotesBridge::*_t)();
            if (*reinterpret_cast<_t *>(func) == static_cast<_t>(&NotesBridge::auth_challenge_changed)) {
                *result = 13;
                return;
            }
        }
    }
#ifndef QT_NO_PROPERTIES
    else if (_c == QMetaObject::ReadProperty) {
        NotesBridge *_t = static_cast<NotesBridge *>(_o);
        Q_UNUSED(_t)
        void *_v = _a[0];
        switch (_id) {
        case 0: *reinterpret_cast< QString*>(_v) = _t->m_currentPageName; break;
        case 1: *reinterpret_cast< QString*>(_v) = _t->m_currentPageGroupPath; break;
        case 2: *reinterpret_cast< QString*>(_v) = _t->m_currentPageFullPath; break;
        case 3: *reinterpret_cast< QString*>(_v) = _t->m_currentPageFilePath; break;
        case 4: *reinterpret_cast< QVariantList*>(_v) = _t->m_currentBlocks; break;
        case 5: *reinterpret_cast< bool*>(_v) = _t->m_isJournalPage; break;
        case 6: *reinterpret_cast< int*>(_v) = _t->m_blocksVersion; break;
        case 7: *reinterpret_cast< QString*>(_v) = _t->m_notesDir; break;
        case 8: *reinterpret_cast< QString*>(_v) = _t->m_searchQuery; break;
        case 9: *reinterpret_cast< QVariantList*>(_v) = _t->m_searchResults; break;
        case 10: *reinterpret_cast< bool*>(_v) = _t->m_searchLoading; break;
        case 11: *reinterpret_cast< QVariantList*>(_v) = _t->m_recentPages; break;
        case 12: *reinterpret_cast< QString*>(_v) = _t->m_groupedTreeJson; break;
        case 13: *reinterpret_cast< int*>(_v) = _t->m_groupDisplayDepth; break;
        case 14: *reinterpret_cast< QVariantList*>(_v) = _t->m_recentJournalLines; break;
        case 15: *reinterpret_cast< QVariantList*>(_v) = _t->m_journalBlocks; break;
        case 16: *reinterpret_cast< bool*>(_v) = _t->m_isLoading; break;
        case 17: *reinterpret_cast< bool*>(_v) = _t->m_dropComments; break;
        case 18: *reinterpret_cast< bool*>(_v) = _t->m_rejectPublicNetworks; break;
        case 19: *reinterpret_cast< QString*>(_v) = _t->m_bindAddress; break;
        case 20: *reinterpret_cast< bool*>(_v) = _t->m_webServerRunning; break;
        case 21: *reinterpret_cast< QString*>(_v) = _t->m_webServerUrl; break;
        case 22: *reinterpret_cast< QString*>(_v) = _t->m_errorMessage; break;
        case 23: *reinterpret_cast< bool*>(_v) = _t->m_initialized; break;
        case 24: *reinterpret_cast< bool*>(_v) = _t->m_authChallengePending; break;
        case 25: *reinterpret_cast< QString*>(_v) = _t->m_authChallengeId; break;
        case 26: *reinterpret_cast< QString*>(_v) = _t->m_authVerificationCode; break;
        default: break;
        }
    } else if (_c == QMetaObject::WriteProperty) {
        NotesBridge *_t = static_cast<NotesBridge *>(_o);
        Q_UNUSED(_t)
        void *_v = _a[0];
        switch (_id) {
        case 0:
            if (_t->m_currentPageName != *reinterpret_cast< QString*>(_v)) {
                _t->m_currentPageName = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->page_changed();
            }
            break;
        case 1:
            if (_t->m_currentPageGroupPath != *reinterpret_cast< QString*>(_v)) {
                _t->m_currentPageGroupPath = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->current_page_group_path_changed();
            }
            break;
        case 2:
            if (_t->m_currentPageFullPath != *reinterpret_cast< QString*>(_v)) {
                _t->m_currentPageFullPath = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->current_page_full_path_changed();
            }
            break;
        case 3:
            if (_t->m_currentPageFilePath != *reinterpret_cast< QString*>(_v)) {
                _t->m_currentPageFilePath = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->page_changed();
            }
            break;
        case 4:
            if (_t->m_currentBlocks != *reinterpret_cast< QVariantList*>(_v)) {
                _t->m_currentBlocks = *reinterpret_cast< QVariantList*>(_v);
                Q_EMIT _t->page_changed();
            }
            break;
        case 5:
            if (_t->m_isJournalPage != *reinterpret_cast< bool*>(_v)) {
                _t->m_isJournalPage = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->page_changed();
            }
            break;
        case 6:
            if (_t->m_blocksVersion != *reinterpret_cast< int*>(_v)) {
                _t->m_blocksVersion = *reinterpret_cast< int*>(_v);
                Q_EMIT _t->page_changed();
            }
            break;
        case 7:
            if (_t->m_notesDir != *reinterpret_cast< QString*>(_v)) {
                _t->m_notesDir = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->page_changed();
            }
            break;
        case 8:
            if (_t->m_searchQuery != *reinterpret_cast< QString*>(_v)) {
                _t->m_searchQuery = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->search_results_changed();
            }
            break;
        case 9:
            if (_t->m_searchResults != *reinterpret_cast< QVariantList*>(_v)) {
                _t->m_searchResults = *reinterpret_cast< QVariantList*>(_v);
                Q_EMIT _t->search_results_changed();
            }
            break;
        case 10:
            if (_t->m_searchLoading != *reinterpret_cast< bool*>(_v)) {
                _t->m_searchLoading = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->loading_changed();
            }
            break;
        case 11:
            if (_t->m_recentPages != *reinterpret_cast< QVariantList*>(_v)) {
                _t->m_recentPages = *reinterpret_cast< QVariantList*>(_v);
                Q_EMIT _t->data_refreshed();
            }
            break;
        case 12:
            if (_t->m_groupedTreeJson != *reinterpret_cast< QString*>(_v)) {
                _t->m_groupedTreeJson = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->data_refreshed();
            }
            break;
        case 13:
            if (_t->m_groupDisplayDepth != *reinterpret_cast< int*>(_v)) {
                _t->m_groupDisplayDepth = *reinterpret_cast< int*>(_v);
                Q_EMIT _t->group_depth_changed();
            }
            break;
        case 14:
            if (_t->m_recentJournalLines != *reinterpret_cast< QVariantList*>(_v)) {
                _t->m_recentJournalLines = *reinterpret_cast< QVariantList*>(_v);
                Q_EMIT _t->data_refreshed();
            }
            break;
        case 15:
            if (_t->m_journalBlocks != *reinterpret_cast< QVariantList*>(_v)) {
                _t->m_journalBlocks = *reinterpret_cast< QVariantList*>(_v);
                Q_EMIT _t->data_refreshed();
            }
            break;
        case 16:
            if (_t->m_isLoading != *reinterpret_cast< bool*>(_v)) {
                _t->m_isLoading = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->loading_changed();
            }
            break;
        case 17:
            if (_t->m_dropComments != *reinterpret_cast< bool*>(_v)) {
                _t->m_dropComments = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->drop_comments_changed();
            }
            break;
        case 18:
            if (_t->m_rejectPublicNetworks != *reinterpret_cast< bool*>(_v)) {
                _t->m_rejectPublicNetworks = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->reject_public_networks_changed();
            }
            break;
        case 19:
            if (_t->m_bindAddress != *reinterpret_cast< QString*>(_v)) {
                _t->m_bindAddress = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->bind_address_changed();
            }
            break;
        case 20:
            if (_t->m_webServerRunning != *reinterpret_cast< bool*>(_v)) {
                _t->m_webServerRunning = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->web_server_status_changed();
            }
            break;
        case 21:
            if (_t->m_webServerUrl != *reinterpret_cast< QString*>(_v)) {
                _t->m_webServerUrl = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->web_server_status_changed();
            }
            break;
        case 22:
            if (_t->m_errorMessage != *reinterpret_cast< QString*>(_v)) {
                _t->m_errorMessage = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->error_occurred(_t->m_errorMessage);
            }
            break;
        case 23:
            if (_t->m_initialized != *reinterpret_cast< bool*>(_v)) {
                _t->m_initialized = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->initialized_changed();
            }
            break;
        case 24:
            if (_t->m_authChallengePending != *reinterpret_cast< bool*>(_v)) {
                _t->m_authChallengePending = *reinterpret_cast< bool*>(_v);
                Q_EMIT _t->auth_challenge_changed();
            }
            break;
        case 25:
            if (_t->m_authChallengeId != *reinterpret_cast< QString*>(_v)) {
                _t->m_authChallengeId = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->auth_challenge_changed();
            }
            break;
        case 26:
            if (_t->m_authVerificationCode != *reinterpret_cast< QString*>(_v)) {
                _t->m_authVerificationCode = *reinterpret_cast< QString*>(_v);
                Q_EMIT _t->auth_challenge_changed();
            }
            break;
        default: break;
        }
    } else if (_c == QMetaObject::ResetProperty) {
    }
#endif // QT_NO_PROPERTIES
}

const QMetaObject NotesBridge::staticMetaObject = {
    { &QObject::staticMetaObject, qt_meta_stringdata_NotesBridge.data,
      qt_meta_data_NotesBridge,  qt_static_metacall, Q_NULLPTR, Q_NULLPTR}
};


const QMetaObject *NotesBridge::metaObject() const
{
    return QObject::d_ptr->metaObject ? QObject::d_ptr->dynamicMetaObject() : &staticMetaObject;
}

void *NotesBridge::qt_metacast(const char *_clname)
{
    if (!_clname) return Q_NULLPTR;
    if (!strcmp(_clname, qt_meta_stringdata_NotesBridge.stringdata0))
        return static_cast<void*>(const_cast< NotesBridge*>(this));
    return QObject::qt_metacast(_clname);
}

int NotesBridge::qt_metacall(QMetaObject::Call _c, int _id, void **_a)
{
    _id = QObject::qt_metacall(_c, _id, _a);
    if (_id < 0)
        return _id;
    if (_c == QMetaObject::InvokeMetaMethod) {
        if (_id < 68)
            qt_static_metacall(this, _c, _id, _a);
        _id -= 68;
    } else if (_c == QMetaObject::RegisterMethodArgumentMetaType) {
        if (_id < 68)
            *reinterpret_cast<int*>(_a[0]) = -1;
        _id -= 68;
    }
#ifndef QT_NO_PROPERTIES
   else if (_c == QMetaObject::ReadProperty || _c == QMetaObject::WriteProperty
            || _c == QMetaObject::ResetProperty || _c == QMetaObject::RegisterPropertyMetaType) {
        qt_static_metacall(this, _c, _id, _a);
        _id -= 27;
    } else if (_c == QMetaObject::QueryPropertyDesignable) {
        _id -= 27;
    } else if (_c == QMetaObject::QueryPropertyScriptable) {
        _id -= 27;
    } else if (_c == QMetaObject::QueryPropertyStored) {
        _id -= 27;
    } else if (_c == QMetaObject::QueryPropertyEditable) {
        _id -= 27;
    } else if (_c == QMetaObject::QueryPropertyUser) {
        _id -= 27;
    }
#endif // QT_NO_PROPERTIES
    return _id;
}

// SIGNAL 0
void NotesBridge::page_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 0, Q_NULLPTR);
}

// SIGNAL 1
void NotesBridge::current_page_group_path_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 1, Q_NULLPTR);
}

// SIGNAL 2
void NotesBridge::current_page_full_path_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 2, Q_NULLPTR);
}

// SIGNAL 3
void NotesBridge::search_results_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 3, Q_NULLPTR);
}

// SIGNAL 4
void NotesBridge::data_refreshed()
{
    QMetaObject::activate(this, &staticMetaObject, 4, Q_NULLPTR);
}

// SIGNAL 5
void NotesBridge::group_depth_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 5, Q_NULLPTR);
}

// SIGNAL 6
void NotesBridge::loading_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 6, Q_NULLPTR);
}

// SIGNAL 7
void NotesBridge::drop_comments_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 7, Q_NULLPTR);
}

// SIGNAL 8
void NotesBridge::reject_public_networks_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 8, Q_NULLPTR);
}

// SIGNAL 9
void NotesBridge::bind_address_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 9, Q_NULLPTR);
}

// SIGNAL 10
void NotesBridge::web_server_status_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 10, Q_NULLPTR);
}

// SIGNAL 11
void NotesBridge::error_occurred(QString _t1)
{
    void *_a[] = { Q_NULLPTR, const_cast<void*>(reinterpret_cast<const void*>(&_t1)) };
    QMetaObject::activate(this, &staticMetaObject, 11, _a);
}

// SIGNAL 12
void NotesBridge::initialized_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 12, Q_NULLPTR);
}

// SIGNAL 13
void NotesBridge::auth_challenge_changed()
{
    QMetaObject::activate(this, &staticMetaObject, 13, Q_NULLPTR);
}
QT_END_MOC_NAMESPACE
