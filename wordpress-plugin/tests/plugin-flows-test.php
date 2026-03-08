<?php

/**
 * Lightweight flow tests for the VeloServe WordPress plugin.
 *
 * This does not boot a full WordPress runtime. It stubs core option/storage
 * functions to validate activation, persistence, registration success/failure.
 */

$GLOBALS['wp_options'] = [];
$GLOBALS['http_mock'] = null;

if (!defined('ABSPATH')) {
    define('ABSPATH', __DIR__ . '/');
}

if (!defined('VELOSERVE_PLUGIN_VERSION')) {
    define('VELOSERVE_PLUGIN_VERSION', '0.1.0');
}
if (!defined('VELOSERVE_OPTION_KEY')) {
    define('VELOSERVE_OPTION_KEY', 'veloserve_settings');
}
if (!defined('VELOSERVE_STATUS_KEY')) {
    define('VELOSERVE_STATUS_KEY', 'veloserve_status');
}

class WP_Error
{
    private $code;
    private $message;

    public function __construct($code, $message)
    {
        $this->code = $code;
        $this->message = $message;
    }

    public function get_error_message()
    {
        return $this->message;
    }
}

function is_wp_error($value)
{
    return $value instanceof WP_Error;
}

function get_option($key, $default = false)
{
    return array_key_exists($key, $GLOBALS['wp_options']) ? $GLOBALS['wp_options'][$key] : $default;
}

function add_option($key, $value)
{
    $GLOBALS['wp_options'][$key] = $value;
    return true;
}

function update_option($key, $value)
{
    $GLOBALS['wp_options'][$key] = $value;
    return true;
}

function home_url($path = '/')
{
    return 'https://example.test' . $path;
}

function get_bloginfo($key)
{
    if ($key === 'name') {
        return 'Test Site';
    }

    if ($key === 'version') {
        return '6.7.0';
    }

    return '';
}

function wp_remote_post($url, $args = [])
{
    if (is_callable($GLOBALS['http_mock'])) {
        return call_user_func($GLOBALS['http_mock'], $url, $args);
    }

    return [
        'response' => ['code' => 201],
        'body' => json_encode(['node_id' => 'node-default']),
    ];
}

function wp_remote_retrieve_response_code($response)
{
    return isset($response['response']['code']) ? (int) $response['response']['code'] : 500;
}

function wp_remote_retrieve_body($response)
{
    return isset($response['body']) ? $response['body'] : '';
}

function current_time($format, $gmt = false)
{
    return '2026-03-07 21:00:00';
}

function sanitize_text_field($value)
{
    return trim((string) $value);
}

function esc_url_raw($value)
{
    return trim((string) $value);
}

function untrailingslashit($value)
{
    return rtrim((string) $value, '/');
}

function wp_json_encode($value)
{
    return json_encode($value);
}

class WP_Post
{
    public $post_status;

    public function __construct($post_status)
    {
        $this->post_status = $post_status;
    }
}

function add_action() {}
function wp_is_post_revision() { return false; }
function get_permalink($post_id) { return 'https://example.test/post/' . $post_id; }

require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-client.php';
require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-admin.php';
require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-plugin.php';

function assert_true($condition, $message)
{
    if (!$condition) {
        fwrite(STDERR, "FAIL: {$message}\n");
        exit(1);
    }
}

function assert_equals($expected, $actual, $message)
{
    if ($expected !== $actual) {
        fwrite(STDERR, "FAIL: {$message}; expected=" . var_export($expected, true) . " actual=" . var_export($actual, true) . "\n");
        exit(1);
    }
}

VeloServe_Plugin::activate();
$settings = get_option('veloserve_settings');
$status = get_option('veloserve_status');
assert_true(is_array($settings), 'Activation should create settings option');
assert_equals(1, $settings['auto_purge'], 'auto_purge default should be enabled');
assert_true(is_array($status), 'Activation should create status option');
assert_equals(false, $status['connected'], 'connected should default to false');

update_option('veloserve_settings', [
    'endpoint_url' => 'https://control.example.test',
    'api_token' => 'secret-token',
    'auto_purge' => 1,
]);

$plugin = VeloServe_Plugin::instance();
$plugin->bootstrap();

$GLOBALS['http_mock'] = function ($url, $args) {
    return [
        'response' => ['code' => 201],
        'body' => json_encode(['node_id' => 'node-123']),
    ];
};

$result = $plugin->register_with_endpoint();
assert_true(!is_wp_error($result), 'Register should succeed on 2xx response');
assert_equals('node-123', get_option('veloserve_status')['node_id'], 'Node id should persist on success');
assert_equals(true, get_option('veloserve_status')['connected'], 'Connected should be true after success');

$GLOBALS['http_mock'] = function ($url, $args) {
    return [
        'response' => ['code' => 500],
        'body' => '{}',
    ];
};

$failure = $plugin->register_with_endpoint();
assert_true(is_wp_error($failure), 'Register should fail on non-2xx response');

$purge_urls = [];
$GLOBALS['http_mock'] = function ($url, $args) use (&$purge_urls) {
    $purge_urls[] = $url;
    return [
        'response' => ['code' => 200],
        'body' => '{}',
    ];
};

$post = new WP_Post('publish');
$plugin->purge_cache_on_content_change(42, $post);
assert_true(count($purge_urls) === 1, 'Content change should trigger one purge request');
assert_true(strpos($purge_urls[0], '/api/v1/cache/purge') !== false, 'Purge should hit cache purge endpoint');

$purge_urls = [];
$plugin->purge_cache_on_switch_theme();
assert_true(count($purge_urls) === 1, 'Theme switch should trigger purge request');

$purge_urls = [];
update_option('veloserve_settings', [
    'endpoint_url' => 'https://control.example.test',
    'api_token' => 'secret-token',
    'auto_purge' => 0,
]);
$plugin->purge_cache_on_content_change(43, $post);
assert_true(count($purge_urls) === 0, 'Content change should not purge when auto_purge is disabled');

VeloServe_Plugin::deactivate();
assert_equals(false, get_option('veloserve_status')['connected'], 'Deactivate should set connected=false');

echo "Plugin flow tests passed.\n";
