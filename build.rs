//! Build script for VeloServe
//!
//! When compiled with `--features php-embed`, this script:
//! 1. Finds PHP installation using php-config
//! 2. Configures linking against libphp
//! 3. Sets up include paths for FFI

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Only run PHP detection if the php-embed feature is enabled
    if env::var("CARGO_FEATURE_PHP_EMBED").is_ok() {
        println!("cargo:rerun-if-changed=build.rs");
        
        setup_php_embed();
        generate_php_bindings();
    }
}

fn setup_php_embed() {
    println!("cargo:warning=Building with PHP embed SAPI support");

    // Get PHP library path
    let lib_dir = get_php_config("--prefix")
        .map(|p| format!("{}/lib", p.trim()))
        .unwrap_or_else(|| "/usr/lib".to_string());
    
    // Also check common locations
    let lib_paths = [
        &lib_dir,
        "/usr/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/local/lib",
    ];

    for path in &lib_paths {
        println!("cargo:rustc-link-search=native={}", path);
    }

    // Link against PHP library
    // Try different library names in order of preference
    let php_version = get_php_config("--version")
        .map(|v| v.trim().split('.').take(2).collect::<Vec<_>>().join("."))
        .unwrap_or_else(|| "8.3".to_string());
    
    let _major_minor = php_version.replace('.', "");
    
    println!("cargo:rustc-link-lib=php{}", php_version);
    
    // Get additional libraries PHP depends on
    if let Some(libs) = get_php_config("--libs") {
        for lib in libs.split_whitespace() {
            if lib.starts_with("-l") {
                let lib_name = &lib[2..];
                println!("cargo:rustc-link-lib={}", lib_name);
            }
        }
    }
    
    // Get PHP include paths (for potential bindgen use)
    if let Some(includes) = get_php_config("--includes") {
        for inc in includes.split_whitespace() {
            if inc.starts_with("-I") {
                let path = &inc[2..];
                println!("cargo:include={}", path);
            }
        }
    }

    // Set environment variable for the crate to know PHP version
    println!("cargo:rustc-env=PHP_VERSION={}", php_version);
    
    println!("cargo:warning=PHP {} embed SAPI configured successfully", php_version);
}

fn get_php_config(arg: &str) -> Option<String> {
    Command::new("php-config")
        .arg(arg)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

/// Generate PHP FFI bindings using bindgen (for embed SAPI)
fn generate_php_bindings() {
    let mut includes = get_php_config("--includes")
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|inc| inc.strip_prefix("-I").map(|s| s.to_string()))
        .collect::<Vec<_>>();

    // Add embed include dir if present (php-config --includes doesn't include sapi/embed)
    if let Some(base) = includes.iter().find(|p| p.contains("/php/")) {
        let embed_dir = format!("{}/sapi/embed", base);
        includes.push(embed_dir);
    }

    // Write a minimal header that pulls in PHP SAPI definitions
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let header_path = out_dir.join("php_bindings.h");
    std::fs::write(
        &header_path,
        r#"
            #include <php.h>
            #include <sapi/embed/php_embed.h>
            #include <SAPI.h>
            #include <php_main.h>
            #include <php_variables.h>
            #include <php_globals.h>
        "#,
    )
    .expect("Failed to write php_bindings.h");

    let mut builder = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_args(
            includes
                .iter()
                .map(|inc| format!("-I{}", inc))
        )
        // Keep only what we need for SAPI embedding
        .allowlist_type("sapi_module_struct")
        .allowlist_type("sapi_request_info")
        .allowlist_type("sapi_headers_struct")
        .allowlist_type("sapi_header_struct")
        .allowlist_type("sapi_header_line")
        .allowlist_type("sapi_globals_struct")
        .allowlist_type("php_stream")
        .allowlist_function("php_embed_init")
        .allowlist_function("php_embed_shutdown")
        .allowlist_var("php_embed_module")
        .allowlist_function("php_execute_script")
        .allowlist_function("php_request_startup")
        .allowlist_function("php_request_shutdown")
        .allowlist_function("php_module_startup")
        .allowlist_function("php_module_shutdown")
        .allowlist_function("zend_eval_string")
        .allowlist_function("zend_eval_stringl")
        .allowlist_function("php_output_start_default")
        .allowlist_function("php_output_get_contents")
        .allowlist_function("php_output_discard")
        .allowlist_function("php_output_end")
        .allowlist_function("php_output_get_length")
        .allowlist_function("sapi_add_header")
        .allowlist_var("php_embed_module")
        .allowlist_var("sapi_globals")
        .allowlist_function("zend_stream_init_filename")
        .allowlist_function("zend_destroy_file_handle")
        .allowlist_type("zend_file_handle")
        .allowlist_type("zend_stream_type")
        .allowlist_type("zend_stream")
        .allowlist_type("zend_mmap")
        .allowlist_type("zend_llist")
        .allowlist_type("zend_llist_element")
        // For $_SERVER registration
        .allowlist_function("php_register_variable")
        .allowlist_function("php_register_variable_safe")
        .allowlist_function("php_import_environment_variables")
        // Avoid generating layout tests to speed up builds and reduce dependencies
        .layout_tests(false)
        .generate_comments(false);

    let bindings = builder
        .generate()
        .expect("Unable to generate PHP bindings");

    let out_path = out_dir.join("php_bindings.rs");
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write PHP bindings");
}

