//! PHP SAPI (Server API) Integration
//!
//! This module provides true PHP embedding using the php-embed SAPI.
//! PHP runs directly inside VeloServe - no external processes!
//!
//! ## Important: Thread Safety
//!
//! PHP embed SAPI is NOT thread-safe. All PHP operations must happen on the
//! same thread that called `php_embed_init`. This module uses a dedicated
//! background thread with channel-based communication to ensure thread safety.
//!
//! ## Usage
//!
//! ```bash
//! # Build with embedded PHP support
//! cargo build --release --features php-embed
//! ```
//!
//! ## Requirements
//!
//! - PHP development files: `sudo apt install php-dev libphp-embed`
//! - Or compile PHP with `--enable-embed`

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Once;
#[cfg(feature = "php-embed")]
use std::sync::mpsc;
#[cfg(feature = "php-embed")]
use std::thread;

use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

#[cfg(feature = "php-embed")]
use super::ffi::bindings as b;
#[cfg(feature = "php-embed")]
use chrono::Utc;
#[cfg(feature = "php-embed")]
use once_cell::sync::OnceCell;
#[cfg(feature = "php-embed")]
use parking_lot::Mutex as ParkingMutex;

// ============================================================================
// PHP SAPI Runtime
// ============================================================================

static PHP_INITIALIZED: AtomicBool = AtomicBool::new(false);
static PHP_INIT_ONCE: Once = Once::new();
static PHP_INIT_ERROR: Mutex<Option<String>> = Mutex::new(None);
#[cfg(feature = "php-embed")]
static PHP_HOOKS_INSTALLED: Once = Once::new();
#[cfg(feature = "php-embed")]
static CAPTURE: OnceCell<ParkingMutex<EmbedCapture>> = OnceCell::new();
#[cfg(feature = "php-embed")]
static EMBED_ARGV_STRS: OnceCell<Vec<CString>> = OnceCell::new();
#[cfg(feature = "php-embed")]
static EMBED_ARGV_PTRS: OnceCell<&'static [usize]> = OnceCell::new();
#[cfg(feature = "php-embed")]
static EMBED_INI: OnceCell<CString> = OnceCell::new();
#[cfg(feature = "php-embed")]
static EMBED_INI_PATH: OnceCell<PathBuf> = OnceCell::new();
#[cfg(feature = "php-embed")]
static REQUEST_CONTEXT: OnceCell<ParkingMutex<RequestContext>> = OnceCell::new();
#[cfg(feature = "php-embed")]
static PHP_ERROR_LOG_PATH: OnceCell<PathBuf> = OnceCell::new();

/// Channel for sending PHP execution requests to the dedicated PHP thread
#[cfg(feature = "php-embed")]
static PHP_WORKER_TX: OnceCell<mpsc::SyncSender<PhpWorkerRequest>> = OnceCell::new();

/// Configuration for PHP embed initialization
#[derive(Clone, Default)]
pub struct PhpEmbedConfig {
    /// Stack limit for PHP (e.g., "16M", "512M")
    pub stack_limit: String,
    /// Path to error log file
    pub error_log: Option<String>,
    /// Whether to display errors in output
    pub display_errors: bool,
    /// Additional INI settings
    pub ini_settings: Vec<String>,
}

/// Request to execute PHP script on the dedicated thread
#[cfg(feature = "php-embed")]
struct PhpWorkerRequest {
    script_path: PathBuf,
    server_vars: HashMap<String, String>,
    get_vars: HashMap<String, String>,
    post_data: Vec<u8>,
    headers: HashMap<String, String>,
    response_tx: mpsc::SyncSender<Result<PhpResponse, String>>,
}

#[cfg(feature = "php-embed")]
#[derive(Default)]
struct EmbedCapture {
    body: Vec<u8>,
    headers: Vec<(String, String)>,
    status: u16,
    last_error: Option<String>,
}

#[cfg(feature = "php-embed")]
#[derive(Default)]
struct RequestContext {
    body: Vec<u8>,
    cursor: usize,
    cookie: Option<CString>,
    /// Server variables for $_SERVER population
    server_vars: HashMap<String, String>,
}

#[cfg(feature = "php-embed")]
unsafe extern "C" fn ub_write_hook(str_: *const c_char, str_length: usize) -> usize {
    if str_.is_null() {
        return 0;
    }
    if let Some(lock) = CAPTURE.get() {
        let slice = std::slice::from_raw_parts(str_ as *const u8, str_length);
        let mut cap = lock.lock();
        cap.body.extend_from_slice(slice);
    }
    str_length
}

#[cfg(feature = "php-embed")]
unsafe extern "C" fn header_handler_hook(
    sapi_header: *mut b::sapi_header_struct,
    op: b::sapi_header_op_enum,
    _sapi_headers: *mut b::sapi_headers_struct,
) -> c_int {
    if sapi_header.is_null() {
        return 0;
    }
    if op == b::sapi_header_op_enum_SAPI_HEADER_ADD
        || op == b::sapi_header_op_enum_SAPI_HEADER_REPLACE
    {
        let line = std::slice::from_raw_parts(
            (*sapi_header).header as *const u8,
            (*sapi_header).header_len,
        );
        if let Ok(s) = std::str::from_utf8(line) {
            let trimmed = s.trim_matches(|c| c == '\r' || c == '\n');
            if let Some(rest) = trimmed
                .strip_prefix("Status:")
                .or_else(|| trimmed.strip_prefix("status:"))
            {
                if let Some(code) = rest.trim().split_whitespace().next() {
                    if let Ok(code) = code.parse::<u16>() {
                        if let Some(lock) = CAPTURE.get() {
                            lock.lock().status = code;
                        }
                    }
                }
            } else if let Some((name, value)) = trimmed.split_once(':') {
                if let Some(lock) = CAPTURE.get() {
                    let mut guard = lock.lock();
                    let header_name = name.trim().to_string();
                    let header_value = value.trim().to_string();
                    
                    if op == b::sapi_header_op_enum_SAPI_HEADER_REPLACE {
                        // REPLACE: Remove existing headers with same name (case-insensitive)
                        // Exception: Set-Cookie headers should always be added, not replaced
                        let name_lower = header_name.to_lowercase();
                        if name_lower != "set-cookie" {
                            guard.headers.retain(|(n, _)| n.to_lowercase() != name_lower);
                        }
                    }
                    guard.headers.push((header_name, header_value));
                }
            }
        }
    }
    0
}

#[cfg(feature = "php-embed")]
unsafe extern "C" fn send_headers_hook(
    sapi_headers: *mut b::sapi_headers_struct,
) -> c_int {
    if let Some(lock) = CAPTURE.get() {
        if !sapi_headers.is_null() {
            let code = (*sapi_headers).http_response_code;
            if code > 0 {
                lock.lock().status = code as u16;
            }
        }
    }
    0
}

#[cfg(feature = "php-embed")]
unsafe extern "C" fn read_post_hook(buffer: *mut c_char, count_bytes: usize) -> usize {
    if buffer.is_null() || count_bytes == 0 {
        return 0;
    }

    if let Some(cell) = REQUEST_CONTEXT.get() {
        let mut ctx = cell.lock();
        
        if ctx.cursor >= ctx.body.len() {
            return 0;
        }

        let remaining = ctx.body.len().saturating_sub(ctx.cursor);
        let to_copy = remaining.min(count_bytes);

        ptr::copy_nonoverlapping(
            ctx.body.as_ptr().add(ctx.cursor),
            buffer as *mut u8,
            to_copy,
        );
        ctx.cursor += to_copy;
        return to_copy;
    }

    0
}

#[cfg(feature = "php-embed")]
unsafe extern "C" fn read_cookies_hook() -> *mut c_char {
    if let Some(cell) = REQUEST_CONTEXT.get() {
        let ctx = cell.lock();
        if let Some(ref cookie) = ctx.cookie {
            return cookie.as_ptr() as *mut c_char;
        }
    }

    std::ptr::null_mut()
}

#[cfg(feature = "php-embed")]
unsafe extern "C" fn log_message_hook(
    message: *const c_char,
    _syslog_type_int: c_int,
) {
    if message.is_null() {
        return;
    }
    let c_str = std::ffi::CStr::from_ptr(message);
    if let Ok(msg) = c_str.to_str() {
        // Log to VeloServe logger
        error!("PHP: {}", msg);
        
        // Capture for response handling
        if let Some(lock) = CAPTURE.get() {
            let mut cap = lock.lock();
            cap.last_error = Some(msg.to_string());
        }
        
        // Write to PHP error log file if configured
        if let Some(log_path) = PHP_ERROR_LOG_PATH.get() {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
            {
                // Format with timestamp similar to PHP's error_log format
                let timestamp = Utc::now().format("[%d-%b-%Y %H:%M:%S UTC]");
                let _ = writeln!(file, "{} {}", timestamp, msg);
            }
        }
    }
}

/// Hook to populate $_SERVER variables
#[cfg(feature = "php-embed")]
unsafe extern "C" fn register_server_variables_hook(track_vars_array: *mut b::_zval_struct) {
    if track_vars_array.is_null() {
        return;
    }

    if let Some(cell) = REQUEST_CONTEXT.get() {
        let ctx = cell.lock();
        for (key, value) in &ctx.server_vars {
            if let (Ok(key_c), Ok(val_c)) = (CString::new(key.as_str()), CString::new(value.as_str())) {
                b::php_register_variable(
                    key_c.as_ptr() as *mut c_char,
                    val_c.as_ptr() as *mut c_char,
                    track_vars_array,
                );
            }
        }
    }
}

#[cfg(feature = "php-embed")]
unsafe fn install_hooks() {
    PHP_HOOKS_INSTALLED.call_once(|| {
        let _ = CAPTURE.get_or_init(|| ParkingMutex::new(EmbedCapture::default()));
        let _ = REQUEST_CONTEXT.get_or_init(|| ParkingMutex::new(RequestContext::default()));
        let module = &raw mut b::php_embed_module;
        (*module).ub_write = Some(ub_write_hook);
        (*module).header_handler = Some(header_handler_hook);
        (*module).send_headers = Some(send_headers_hook);
        (*module).read_post = Some(read_post_hook);
        (*module).read_cookies = Some(read_cookies_hook);
        (*module).log_message = Some(log_message_hook);
        (*module).register_server_variables = Some(register_server_variables_hook);
    });
}
/// PHP SAPI Runtime Manager
///
/// Manages the embedded PHP runtime lifecycle.
/// PHP embed is NOT thread-safe, so all PHP operations run on a dedicated thread.
/// Only one instance should exist per process.
pub struct PhpSapi {
    /// Whether this instance successfully initialized PHP
    initialized: bool,
    /// Request counter for statistics
    request_count: AtomicU64,
    /// Output buffer for capturing PHP output
    output_buffer: Mutex<Vec<u8>>,
}

/// Run the PHP worker thread that handles all PHP execution
#[cfg(feature = "php-embed")]
fn run_php_worker(
    rx: mpsc::Receiver<PhpWorkerRequest>,
    config: PhpEmbedConfig,
) {
    info!("PHP worker thread starting...");

    unsafe {
        install_hooks();

        // Provide minimal argv with a safer stack limit for WordPress
        let limit = config.stack_limit.as_str();
        let argv_strs = EMBED_ARGV_STRS.get_or_init(|| {
            vec![
                CString::new("veloserve-embed").unwrap(),
                CString::new("-d").unwrap(),
                CString::new(format!("zend.max_allowed_stack_size={}", limit)).unwrap(),
            ]
        });

        // Build argv pointers and add null terminator; keep alive in OnceCell
        let argv_ptrs = EMBED_ARGV_PTRS.get_or_init(|| {
            let mut v: Vec<usize> = argv_strs
                .iter()
                .map(|s| s.as_ptr() as usize)
                .collect();
            v.push(std::ptr::null_mut::<c_char>() as usize);
            Box::leak(v.into_boxed_slice()) as &'static [usize]
        });

        // Build INI settings string
        let ini_cstr = EMBED_INI.get_or_init(|| {
            let mut ini_parts = vec![
                format!("zend.max_allowed_stack_size={}", limit),
                "opcache.enable=0".to_string(),
                "opcache.enable_cli=0".to_string(),
                "opcache.jit=0".to_string(),
                "opcache.jit_buffer_size=0".to_string(),
                "pcre.jit=0".to_string(),
                "realpath_cache_size=0".to_string(),
                "realpath_cache_ttl=0".to_string(),
                "log_errors=On".to_string(),
            ];
            
            // Error display setting
            if config.display_errors {
                ini_parts.push("display_errors=On".to_string());
                ini_parts.push("display_startup_errors=On".to_string());
            } else {
                ini_parts.push("display_errors=Off".to_string());
                ini_parts.push("display_startup_errors=Off".to_string());
            }
            
            // Error log setting
            if let Some(ref error_log) = config.error_log {
                ini_parts.push(format!("error_log={}", error_log));
                info!("PHP error log configured: {}", error_log);
                // Store path for log_message_hook to use
                let _ = PHP_ERROR_LOG_PATH.set(PathBuf::from(error_log));
            }
            
            // Add any additional custom INI settings
            for setting in &config.ini_settings {
                ini_parts.push(setting.clone());
            }
            
            CString::new(ini_parts.join("\n")).unwrap()
        });

        // argc should not include the null terminator
        let argc = (argv_ptrs.len().saturating_sub(1)) as c_int;
        let argv = argv_ptrs.as_ptr() as *mut *mut c_char;

        // Assign ini_entries before init
        let module = &raw mut b::php_embed_module;
        (*module).ini_entries = ini_cstr.as_ptr();

        // Write temp ini file to force settings in embed
        let ini_path = EMBED_INI_PATH.get_or_init(|| {
            let mut p = std::env::temp_dir();
            p.push("veloserve-embed.ini");
            let content = ini_cstr.to_bytes();
            let _ = std::fs::write(&p, content);
            p
        });
        (*module).php_ini_path_override = ini_path
            .as_os_str()
            .to_str()
            .map(|s| CString::new(s).unwrap().into_raw())
            .unwrap_or(std::ptr::null_mut());

        let result = b::php_embed_init(argc, argv);

        if result != 0 {
            let err = format!("php_embed_init failed with code: {}", result);
            error!("{}", err);
            *PHP_INIT_ERROR.lock() = Some(err);
            return;
        }

        // CRITICAL: php_embed_init() calls php_request_startup() internally,
        // leaving an active "boot" request. We MUST shut it down before
        // processing our own requests, otherwise request state is inconsistent
        // and POST data parsing won't work properly.
        b::php_request_shutdown(std::ptr::null_mut());
        debug!("Shut down initial boot request from php_embed_init");

        PHP_INITIALIZED.store(true, Ordering::SeqCst);
        info!("PHP embed SAPI initialized on worker thread");

        // Process requests from the channel
        while let Ok(req) = rx.recv() {
            let result = execute_script_on_thread(
                &req.script_path,
                &req.server_vars,
                &req.get_vars,
                &req.post_data,
                &req.headers,
            );
            let _ = req.response_tx.send(result);
        }

        info!("PHP worker thread shutting down...");
        b::php_embed_shutdown();
    }
}

/// Execute a script on the PHP worker thread (called from within the worker)
#[cfg(feature = "php-embed")]
unsafe fn execute_script_on_thread(
    script_path: &Path,
    server_vars: &HashMap<String, String>,
    get_vars: &HashMap<String, String>,
    post_data: &[u8],
    headers: &HashMap<String, String>,
) -> Result<PhpResponse, String> {
    let script_path_str = script_path.to_string_lossy();
    let c_script_path = CString::new(script_path_str.as_ref())
        .map_err(|e| format!("Invalid script path: {}", e))?;

    debug!("PHP worker executing script: {}", script_path_str);

    // Reset capture buffer
    let cap_lock = CAPTURE.get_or_init(|| ParkingMutex::new(EmbedCapture::default()));
    {
        let mut cap = cap_lock.lock();
        cap.body.clear();
        cap.headers.clear();
        cap.status = 200;
        cap.last_error = None;
    }

    // Prepare CStrings for request info - keep them alive until request ends
    let mut keep_alive: Vec<CString> = Vec::new();

    let method = server_vars
        .get("REQUEST_METHOD")
        .map(|s| s.as_str())
        .unwrap_or("GET");
    let method_c = CString::new(method).unwrap();
    keep_alive.push(method_c);

    let uri = server_vars
        .get("REQUEST_URI")
        .map(|s| s.as_str())
        .unwrap_or("/");
    let uri_c = CString::new(uri).unwrap();
    keep_alive.push(uri_c);

    let query = server_vars
        .get("QUERY_STRING")
        .map(|s| s.as_str())
        .unwrap_or("");
    let query_c = CString::new(query).unwrap();
    keep_alive.push(query_c);

    let path_translated = server_vars
        .get("SCRIPT_FILENAME")
        .map(|s| s.as_str())
        .unwrap_or(script_path_str.as_ref());
    let path_c = CString::new(path_translated).unwrap();
    keep_alive.push(path_c);

    let content_type = headers
        .get("content-type")
        .or_else(|| headers.get("Content-Type"))
        .map(|s| s.as_str())
        .unwrap_or("application/x-www-form-urlencoded");
    let content_type_c = CString::new(content_type).unwrap();
    keep_alive.push(content_type_c);

    // Capture cookies (if any) so PHP can populate $_COOKIE
    let cookie_header = headers
        .get("cookie")
        .or_else(|| headers.get("Cookie"))
        .cloned();
    let cookie_c = cookie_header.and_then(|c| CString::new(c).ok());

    let argv0_c = CString::new("veloserve-embed").unwrap();
    keep_alive.push(argv0_c);

    // Save request context so hooks can access it during php_request_startup
    if let Some(ctx_cell) = REQUEST_CONTEXT.get() {
        let mut ctx = ctx_cell.lock();
        ctx.body.clear();
        ctx.body.extend_from_slice(post_data);
        ctx.cursor = 0;
        ctx.cookie = cookie_c.clone();
        // Store server_vars for the register_server_variables hook
        ctx.server_vars = server_vars.clone();
    }

    // Best-effort populate environment for the request
    for (key, value) in server_vars {
        std::env::set_var(key, value);
    }
    for (key, value) in get_vars {
        let env_key = format!("GET_{}", key);
        std::env::set_var(env_key, value);
    }
    for (key, value) in headers {
        let env_key = format!("HTTP_{}", key.to_uppercase().replace('-', "_"));
        std::env::set_var(env_key, value);
    }

    // IMPORTANT: Set content_type and content_length BEFORE php_request_startup
    // PHP parses POST data during request startup based on these values
    let sg = &raw mut b::sapi_globals;
    (*sg).request_info.request_method = keep_alive[0].as_ptr();
    (*sg).request_info.content_type = keep_alive[4].as_ptr();
    (*sg).request_info.content_length = post_data.len() as b::zend_long;

    // Expose cookies to PHP BEFORE request startup
    (*sg).request_info.cookie_data = cookie_c
        .as_ref()
        .map(|c| c.as_ptr() as *mut c_char)
        .unwrap_or(std::ptr::null_mut());

    // Reset post reading state BEFORE php_request_startup
    (*sg).read_post_bytes = 0;
    (*sg).post_read = 0;

    debug!(
        "Setting request_info: method={}, content_type={}, content_length={}",
        method,
        content_type,
        post_data.len()
    );
    

    // Start per-request lifecycle - php_request_startup will parse POST data
    debug!("Calling php_request_startup...");
    let startup_result = b::php_request_startup();
    debug!("php_request_startup returned: {}", startup_result);
    if startup_result != 0 {
        return Err(format!("php_request_startup failed with code: {}", startup_result));
    }

    // Set remaining request_info fields AFTER php_request_startup
    (*sg).request_info.request_uri = keep_alive[1].as_ptr() as *mut c_char;
    (*sg).request_info.query_string = keep_alive[2].as_ptr() as *mut c_char;
    (*sg).request_info.path_translated = keep_alive[3].as_ptr() as *mut c_char;
    (*sg).request_info.headers_only = false;
    (*sg).request_info.no_headers = false;
    (*sg).request_info.headers_read = false;
    (*sg).request_info.argv0 = keep_alive[5].as_ptr() as *mut c_char;
    (*sg).request_info.proto_num = 1001; // HTTP/1.1
    (*sg).sapi_headers.http_response_code = 200;

    // Ensure cwd is the script directory for relative includes
    if let Some(parent) = script_path.parent() {
        let _ = std::env::set_current_dir(parent);
        debug!("Changed cwd to: {:?}", parent);
    }

    // Start output buffering
    debug!("Starting output buffering...");
    b::php_output_start_default();

    // Bootstrap: Populate $_GET and $_POST for embed SAPI
    // PHP's embed SAPI doesn't automatically parse query strings or POST data
    {
        debug!("Running SAPI bootstrap...");
        let bootstrap_code = CString::new(r#"
            // Parse query string into $_GET if not already done
            if (isset($_SERVER['QUERY_STRING']) && !empty($_SERVER['QUERY_STRING'])) {
                $_GET = [];
                parse_str($_SERVER['QUERY_STRING'], $_GET);
            }
            
            // Parse POST data if content type is form-urlencoded
            if ($_SERVER['REQUEST_METHOD'] === 'POST' && empty($_POST) && !empty($_SERVER['CONTENT_TYPE'])) {
                $ct = $_SERVER['CONTENT_TYPE'];
                if (stripos($ct, 'application/x-www-form-urlencoded') !== false) {
                    $raw = file_get_contents('php://input');
                    if (!empty($raw)) {
                        parse_str($raw, $_POST);
                    }
                }
            }
            
            // Rebuild $_REQUEST
            $_REQUEST = array_merge($_GET ?? [], $_POST ?? [], $_COOKIE ?? []);
        "#).unwrap();
        let bootstrap_name = CString::new("veloserve_bootstrap").unwrap();
        let _ = b::zend_eval_string(
            bootstrap_code.as_ptr(),
            std::ptr::null_mut(),
            bootstrap_name.as_ptr(),
        );
    }

    // Create file handle for the script
    debug!("Creating file handle for: {}", c_script_path.to_string_lossy());
    let mut file_handle: b::zend_file_handle = std::mem::zeroed();
    b::zend_stream_init_filename(&mut file_handle, c_script_path.as_ptr());
    debug!("File handle type: {}", file_handle.type_);

    // Execute the script
    debug!("Calling php_execute_script...");
    let success = b::php_execute_script(&mut file_handle);
    debug!("php_execute_script returned: {}", success);

    // Get output buffer contents
    let mut output_zval: b::_zval_struct = std::mem::zeroed();
    b::php_output_get_contents(&mut output_zval);

    // End output buffering
    b::php_output_end();

    // Clean up file handle
    b::zend_destroy_file_handle(&mut file_handle);

    // Extract buffered output by reading zend_string from zval
    let mut body: Vec<u8> = Vec::new();
    let mut status_code: u16 = 200;

    let zs = output_zval.value.str_;
    if !zs.is_null() {
        let len = (*zs).len as usize;
        let ptr = (*zs).val.as_ptr() as *const u8;
        if !ptr.is_null() && len > 0 {
            body.extend_from_slice(std::slice::from_raw_parts(ptr, len));
        }
    }

    // End the request
    b::php_request_shutdown(std::ptr::null_mut());
    if let Some(ctx_cell) = REQUEST_CONTEXT.get() {
        let mut ctx = ctx_cell.lock();
        ctx.body.clear();
        ctx.cursor = 0;
        ctx.cookie = None;
        ctx.server_vars.clear();
    }

    // Pick up status from SG if set
    let sg = &raw mut b::sapi_globals;
    if (*sg).sapi_headers.http_response_code > 0 {
        status_code = (*sg).sapi_headers.http_response_code as u16;
    }

    // Merge captured headers/body from hooks
    let cap_lock = CAPTURE
        .get_or_init(|| ParkingMutex::new(EmbedCapture::default()));
    let cap = cap_lock.lock();
    if !cap.body.is_empty() {
        body = cap.body.clone();
    }
    // Use Vec to preserve multiple headers with the same name (e.g., Set-Cookie)
    let resp_headers: Vec<(String, String)> = cap.headers.clone();
    
    // Debug: Log captured headers
    debug!("Captured {} headers:", resp_headers.len());
    for (name, value) in &resp_headers {
        debug!("  {}: {}", name, if value.len() > 60 { &value[..60] } else { value });
    }
    
    // Update status_code from capture if it was set (e.g., via Status header)
    if cap.status != 200 {
        status_code = cap.status;
    }

    // Consider the request successful if:
    // 1. php_execute_script returned true, OR
    // 2. We got a valid HTTP response (redirect, error page, etc.) even if script called exit()
    // 
    // Many PHP apps (WordPress, Laravel, etc.) call exit() after sending headers/redirects,
    // which causes php_execute_script to return false even though the script executed correctly.
    let has_valid_response = status_code != 200 || !body.is_empty() || !resp_headers.is_empty();
    
    if success || has_valid_response {
        debug!("PHP script completed: success={}, status={}, body_len={}, headers={}", 
               success, status_code, body.len(), resp_headers.len());
        Ok(PhpResponse {
            body,
            headers: resp_headers,
            status_code,
        })
    } else {
        // Get the last error from the capture buffer
        let error_msg = cap.last_error.clone().unwrap_or_else(|| "Unknown error".to_string());
        Err(format!("PHP script execution failed: {}", error_msg))
    }
}

impl PhpSapi {
    /// Create a new PHP SAPI instance
    pub fn new() -> Self {
        Self {
            initialized: false,
            request_count: AtomicU64::new(0),
            output_buffer: Mutex::new(Vec::with_capacity(64 * 1024)), // 64KB initial
        }
    }

    /// Initialize the embedded PHP runtime
    ///
    /// This spawns a dedicated thread for PHP execution since PHP embed
    /// is not thread-safe - all PHP operations must happen on the same
    /// thread that called php_embed_init.
    #[cfg(feature = "php-embed")]
    pub fn initialize(&mut self, config: PhpEmbedConfig) -> Result<(), String> {
        PHP_INIT_ONCE.call_once(|| {
            info!("Initializing PHP embed SAPI with dedicated worker thread...");

            // Create a bounded channel for sending work to the PHP thread
            let (tx, rx) = mpsc::sync_channel::<PhpWorkerRequest>(32);

            // Store the sender globally
            let _ = PHP_WORKER_TX.set(tx);

            // Spawn the dedicated PHP worker thread
            thread::Builder::new()
                .name("php-embed-worker".to_string())
                .spawn(move || {
                    run_php_worker(rx, config);
                })
                .expect("Failed to spawn PHP worker thread");

            // Give the worker thread time to initialize
            std::thread::sleep(std::time::Duration::from_millis(100));
        });

        // Check if initialization was successful
        if PHP_INITIALIZED.load(Ordering::SeqCst) {
            self.initialized = true;
            Ok(())
        } else {
            let error = PHP_INIT_ERROR.lock().clone()
                .unwrap_or_else(|| "Unknown PHP initialization error".to_string());
            Err(error)
        }
    }

    /// Fallback when php-embed feature is not enabled
    #[cfg(not(feature = "php-embed"))]
    pub fn initialize(&mut self) -> Result<(), String> {
        Err("PHP embed SAPI not compiled. Build with: cargo build --features php-embed".to_string())
    }

    /// Execute a PHP script and return its output
    ///
    /// This sends the execution request to the dedicated PHP worker thread
    /// and waits for the response.
    ///
    /// # Arguments
    /// * `script_path` - Path to the PHP file
    /// * `server_vars` - $_SERVER variables
    /// * `get_vars` - $_GET query parameters
    /// * `post_data` - Raw POST body
    /// * `headers` - HTTP headers
    ///
    /// # Returns
    /// A tuple of (output_body, response_headers)
    #[cfg(feature = "php-embed")]
    pub fn execute_script(
        &self,
        script_path: &Path,
        server_vars: &HashMap<String, String>,
        get_vars: &HashMap<String, String>,
        post_data: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<PhpResponse, String> {
        if !self.initialized {
            return Err("PHP SAPI not initialized".to_string());
        }

        self.request_count.fetch_add(1, Ordering::Relaxed);

        debug!("Sending PHP request to worker thread: {}", script_path.display());

        // Get the worker channel
        let tx = PHP_WORKER_TX.get()
            .ok_or_else(|| "PHP worker thread not initialized".to_string())?;

        // Create a response channel for this request
        let (response_tx, response_rx) = mpsc::sync_channel(1);

        // Build the request
        let request = PhpWorkerRequest {
            script_path: script_path.to_path_buf(),
            server_vars: server_vars.clone(),
            get_vars: get_vars.clone(),
            post_data: post_data.to_vec(),
            headers: headers.clone(),
            response_tx,
        };

        // Send request to worker thread
        tx.send(request)
            .map_err(|e| format!("Failed to send request to PHP worker: {}", e))?;

        // Wait for response (with timeout)
        response_rx
            .recv_timeout(std::time::Duration::from_secs(300))
            .map_err(|e| format!("Timeout waiting for PHP response: {}", e))?
    }

    /// Execute PHP code string
    #[cfg(feature = "php-embed")]
    pub fn eval_string(&self, code: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("PHP SAPI not initialized".to_string());
        }

        let c_code = CString::new(code)
            .map_err(|e| format!("Invalid PHP code: {}", e))?;
        let c_name = CString::new("<eval>").unwrap();

        unsafe {
            b::php_output_start_default();

            let mut retval: b::_zval_struct = std::mem::zeroed();
            // zend_eval_string may not be available in embed on all builds; check return code
            let result = b::zend_eval_string(
                c_code.as_ptr(),
                &mut retval,
                c_name.as_ptr(),
            );

            let mut output_zval: b::_zval_struct = std::mem::zeroed();
            b::php_output_get_contents(&mut output_zval);
            b::php_output_end();

            if result == 0 {
                // TODO: Convert output_zval to string
                Ok(String::new())
            } else {
                Err("PHP eval failed".to_string())
            }
        }
    }

    #[cfg(not(feature = "php-embed"))]
    pub fn execute_script(
        &self,
        _script_path: &Path,
        _server_vars: &HashMap<String, String>,
        _get_vars: &HashMap<String, String>,
        _post_data: &[u8],
        _headers: &HashMap<String, String>,
    ) -> Result<PhpResponse, String> {
        Err("PHP embed not available".to_string())
    }

    #[cfg(not(feature = "php-embed"))]
    pub fn eval_string(&self, _code: &str) -> Result<String, String> {
        Err("PHP embed not available".to_string())
    }

    /// Check if PHP SAPI is initialized and available
    pub fn is_available(&self) -> bool {
        self.initialized
    }

    /// Get total request count
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({
            "mode": "sapi",
            "initialized": self.initialized,
            "request_count": self.request_count(),
            "feature_enabled": cfg!(feature = "php-embed"),
        })
    }
}

impl Default for PhpSapi {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PhpSapi {
    fn drop(&mut self) {
        #[cfg(feature = "php-embed")]
        if self.initialized && PHP_INITIALIZED.load(Ordering::SeqCst) {
            info!("Shutting down PHP embed SAPI...");
            unsafe {
                b::php_embed_shutdown();
            }
            PHP_INITIALIZED.store(false, Ordering::SeqCst);
            info!("PHP embed SAPI shutdown complete");
        }
    }
}

// ============================================================================
// PHP Response
// ============================================================================

/// Response from PHP script execution
#[derive(Debug, Clone)]
pub struct PhpResponse {
    /// Response body
    pub body: Vec<u8>,
    /// Response headers (Vec to preserve multiple headers with same name, e.g., Set-Cookie)
    pub headers: Vec<(String, String)>,
    /// HTTP status code
    pub status_code: u16,
}

impl PhpResponse {
    /// Create a new PHP response
    pub fn new() -> Self {
        Self {
            body: Vec::new(),
            headers: Vec::new(),
            status_code: 200,
        }
    }

    /// Parse raw PHP output (headers + body)
    pub fn from_raw_output(output: &[u8]) -> Self {
        // Find header/body separator (double CRLF)
        let separator = b"\r\n\r\n";
        if let Some(pos) = output.windows(4).position(|w| w == separator) {
            let headers_bytes = &output[..pos];
            let body = output[pos + 4..].to_vec();

            let mut headers = Vec::new();
            let mut status_code = 200;

            // Parse headers
            let headers_str = String::from_utf8_lossy(headers_bytes);
            for line in headers_str.lines() {
                if line.starts_with("Status:") {
                    // Parse status line: "Status: 404 Not Found"
                    if let Some(code_str) = line.strip_prefix("Status:").map(|s| s.trim()) {
                        if let Some(code) = code_str.split_whitespace().next() {
                            status_code = code.parse().unwrap_or(200);
                        }
                    }
                } else if let Some((name, value)) = line.split_once(':') {
                    headers.push((name.trim().to_string(), value.trim().to_string()));
                }
            }

            Self {
                body,
                headers,
                status_code,
            }
        } else {
            // No headers, entire output is body
            Self {
                body: output.to_vec(),
                headers: Vec::new(),
                status_code: 200,
            }
        }
    }
}

impl Default for PhpResponse {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_sapi_creation() {
        let sapi = PhpSapi::new();
        assert!(!sapi.is_available());
        assert_eq!(sapi.request_count(), 0);
    }

    #[test]
    fn test_php_response_parsing() {
        let raw = b"Content-Type: text/html\r\nStatus: 200 OK\r\n\r\n<html>Hello</html>";
        let response = PhpResponse::from_raw_output(raw);

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, b"<html>Hello</html>");
        assert_eq!(response.headers.get("Content-Type"), Some(&"text/html".to_string()));
    }

    #[test]
    fn test_php_response_no_headers() {
        let raw = b"Hello World";
        let response = PhpResponse::from_raw_output(raw);

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, b"Hello World");
        assert!(response.headers.is_empty());
    }

    #[test]
    fn test_php_response_404() {
        let raw = b"Status: 404 Not Found\r\nContent-Type: text/html\r\n\r\nNot Found";
        let response = PhpResponse::from_raw_output(raw);

        assert_eq!(response.status_code, 404);
    }
}
