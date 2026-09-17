/* notesplusplus_core.h — C FFI header for notesplusplus-core
 *
 * Memory ownership rules:
 * - Every char* returned is Rust-allocated. Caller frees via notes_core_free_string().
 * - Opaque handles (*mut T) are created by _new/_open, freed by _free.
 * - Input const char* parameters are borrowed; FFI copies internally.
 * - Return int: 0 = success, negative = error.
 */

#ifndef NOTESPLUSPLUS_CORE_H
#define NOTESPLUSPLUS_CORE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle types */
typedef struct AppPaths    AppPaths;
typedef struct FfiSearchEngine FfiSearchEngine;
typedef struct FfiAgentSession FfiAgentSession;
typedef struct FfiSttDownload FfiSttDownload;
typedef struct HttpServerHandle HttpServerHandle;

/* ------------------------------------------------------------------ */
/* String ownership — caller MUST call this on every returned char*    */
/* ------------------------------------------------------------------ */
void notes_core_free_string(char* s);

/* ------------------------------------------------------------------ */
/* Constants                                                           */
/* ------------------------------------------------------------------ */
char* notes_core_const_db_filename(void);
char* notes_core_const_journal_filename(void);
char* notes_core_const_default_ai_endpoint(void);
char* notes_core_const_default_ai_model(void);
char* notes_core_const_default_system_prompt(void);
uint16_t notes_core_const_default_server_port(void);

/* ------------------------------------------------------------------ */
/* AppPaths                                                            */
/* ------------------------------------------------------------------ */
AppPaths* notes_core_app_paths_new(void);
void      notes_core_app_paths_free(AppPaths* p);
char*     notes_core_app_paths_data_dir(const AppPaths* p);
char*     notes_core_app_paths_notes_dir(const AppPaths* p);
char*     notes_core_app_paths_db_path(const AppPaths* p);

/* ------------------------------------------------------------------ */
/* Database                                                            */
/* ------------------------------------------------------------------ */
/* Returns NULL on failure */
void* notes_core_db_open(const char* db_path);
void  notes_core_db_close(void* conn);

/* ------------------------------------------------------------------ */
/* Page CRUD                                                           */
/* ------------------------------------------------------------------ */
char* notes_core_page_get_source(const char* notes_dir, const char* name);
int   notes_core_page_save_source(void* conn, const char* notes_dir, const char* name, const char* content);
int   notes_core_page_create(void* conn, const char* notes_dir, const char* name);
int   notes_core_page_delete(void* conn, const char* notes_dir, const char* name);
int   notes_core_page_rename(void* conn, const char* notes_dir, const char* name, const char* new_title);
int   notes_core_page_move(void* conn, const char* notes_dir, const char* source_name, const char* target_group);
char* notes_core_page_extract_title(const char* content, const char* fallback);
int   notes_core_rebuild_index(void* conn, const char* notes_dir);
char* notes_core_recent_pages_json(void* conn, int limit);

/* ------------------------------------------------------------------ */
/* Block parsing & rendering                                           */
/* ------------------------------------------------------------------ */
char* notes_core_parse_blocks_json(const char* adoc, int drop_comments);
char* notes_core_page_parse_and_render_blocks_json(
    const char* adoc_content, const char* notes_dir,
    int drop_comments, const char* theme_json, const char* options_json);
char* notes_core_render_qt_block_json(
    const char* block_json, int index,
    const char* theme_json, const char* options_json);
int   notes_core_page_save_block(
    void* conn, const char* notes_dir, const char* page_path,
    int index, int count, const char* raw_text, int drop_comments);
int   notes_core_page_toggle_checkbox(
    void* conn, const char* notes_dir, const char* page_path,
    int block_index, const char* item_path);

/* ------------------------------------------------------------------ */
/* Groups                                                              */
/* ------------------------------------------------------------------ */
char* notes_core_groups_flat_json(void* conn);
int   notes_core_group_create(void* conn, const char* notes_dir, const char* parent, const char* name);
int   notes_core_group_rename(void* conn, const char* notes_dir, const char* old_path, const char* new_name);
int   notes_core_group_delete(void* conn, const char* notes_dir, const char* path, int recursive);
int   notes_core_group_toggle_collapsed(void* conn, const char* path);
int   notes_core_group_set_note_sort(void* conn, const char* path, int sort_order);
int   notes_core_group_get_note_sort(void* conn, const char* path);
char* notes_core_build_group_tree_json(
    void* conn, const char* notes_dir, int depth,
    int drop_comments, const char* theme_json, const char* options_json);
char* notes_core_load_main_page_data_json(
    void* conn, const char* notes_dir, int depth, int drop_comments);

/* ------------------------------------------------------------------ */
/* Search (background + poll)                                          */
/* ------------------------------------------------------------------ */
FfiSearchEngine* notes_core_search_new(void);
int   notes_core_search_start(FfiSearchEngine* engine, const char* db_path, const char* query);
/* Returns: 0=running, 1=ready, -1=error. On ready, *out_results_json is set. */
int   notes_core_search_poll(FfiSearchEngine* engine, char** out_results_json);
void  notes_core_search_free(FfiSearchEngine* engine);

/* ------------------------------------------------------------------ */
/* Journal                                                             */
/* ------------------------------------------------------------------ */
char* notes_core_journal_today(const char* notes_dir);
int   notes_core_journal_append(void* conn, const char* notes_dir, const char* text, int is_task);
char* notes_core_journal_recent_lines(const char* notes_dir, int limit);

/* ------------------------------------------------------------------ */
/* HTML export                                                         */
/* ------------------------------------------------------------------ */
char* notes_core_export_html5(
    const char* notes_dir, const char* rel_path,
    const char* full_path, const char* output_path);
char* notes_core_export_all_html5(const char* notes_dir, const char* output_dir);

/* ------------------------------------------------------------------ */
/* Web server                                                          */
/* ------------------------------------------------------------------ */
HttpServerHandle* notes_core_server_start(
    const char* notes_dir, const char* db_path, const char* backup_dir,
    uint16_t port, const char* config_json);
void    notes_core_server_stop(HttpServerHandle* handle);
int     notes_core_server_is_running(const HttpServerHandle* handle);
uint16_t notes_core_server_port(const HttpServerHandle* handle);
char*   notes_core_server_urls_json(const HttpServerHandle* handle);

/* ------------------------------------------------------------------ */
/* TLS                                                                 */
/* ------------------------------------------------------------------ */
char* notes_core_server_tls_install(
    const char* cert_path, const char* key_path,
    const char* cert_pem, const char* key_pem);
char* notes_core_server_tls_reset(const char* cert_path, const char* key_path);
int   notes_core_server_tls_is_custom(const char* cert_path);

/* ------------------------------------------------------------------ */
/* Auth challenge (pending approval from browser login)                */
/* ------------------------------------------------------------------ */
char* notes_core_server_auth_take_challenge(const HttpServerHandle* handle);
int   notes_core_server_auth_approve(const HttpServerHandle* handle, const char* challenge_id);
int   notes_core_server_auth_deny(const HttpServerHandle* handle, const char* challenge_id);

/* ------------------------------------------------------------------ */
/* Agent session (background + poll)                                   */
/* ------------------------------------------------------------------ */
typedef void (*FfiTokenCallback)(void* user_data, const char* token, int is_done);
typedef void (*FfiStatusCallback)(void* user_data, const char* status, const char* detail);

FfiAgentSession* notes_core_agent_new(
    const char* notes_dir, const char* db_path,
    const char* backup_dir, const char* config_json);
int   notes_core_agent_send(FfiAgentSession* ffi, const char* prompt);
int   notes_core_agent_send_streaming(
    FfiAgentSession* ffi, const char* prompt,
    FfiTokenCallback callback, void* user_data);
char* notes_core_agent_poll_streaming(FfiAgentSession* ffi);
/* Returns: 0=running, 1=ready, -1=error. On ready, *out_json is set. */
int   notes_core_agent_poll(FfiAgentSession* ffi, char** out_json);
int   notes_core_agent_confirm(FfiAgentSession* ffi, int approved);
int   notes_core_agent_confirm_streaming(
    FfiAgentSession* ffi, int approved,
    FfiTokenCallback callback, void* user_data);
char* notes_core_agent_send_streaming_direct(
    FfiAgentSession* ffi, const char* prompt,
    FfiTokenCallback callback, FfiStatusCallback status_callback, void* user_data);
char* notes_core_agent_confirm_streaming_direct(
    FfiAgentSession* ffi, int approved,
    FfiTokenCallback callback, FfiStatusCallback status_callback, void* user_data);
char* notes_core_agent_undo_direct(FfiAgentSession* ffi);
char* notes_core_agent_undo(FfiAgentSession* ffi);
void  notes_core_agent_configure(FfiAgentSession* ffi, const char* config_json);
void  notes_core_agent_reset_session(
    FfiAgentSession* ffi, const char* context_filename,
    const char* context_content, const char* extra_context);
void  notes_core_agent_free(FfiAgentSession* ffi);

/* ------------------------------------------------------------------ */
/* STT (Speech-to-Text)                                                */
/* ------------------------------------------------------------------ */
char* notes_core_stt_transcribe(const char* model_path, const char* wav_path);
char* notes_core_stt_model_catalog_json(const char* models_dir, const char* active_model_id);
int   notes_core_stt_model_delete(const char* models_dir, const char* model_id);
FfiSttDownload* notes_core_stt_download_start(const char* models_dir, const char* model_id);
/* Returns: 0=running, 1=done, -1=error. */
int   notes_core_stt_download_poll(FfiSttDownload* handle, double* out_progress, char** out_result);
void  notes_core_stt_download_cancel(FfiSttDownload* handle);
void  notes_core_stt_download_free(FfiSttDownload* handle);

/* ------------------------------------------------------------------ */
/* Utilities                                                           */
/* ------------------------------------------------------------------ */
char* notes_core_get_network_interfaces_json(void);
char* notes_core_fetch_url(const char* url);

#ifdef __cplusplus
}
#endif

#endif /* NOTESPLUSPLUS_CORE_H */
