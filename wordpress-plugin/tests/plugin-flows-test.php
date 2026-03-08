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
if (!defined('VELOSERVE_IMAGE_QUEUE_KEY')) {
    define('VELOSERVE_IMAGE_QUEUE_KEY', 'veloserve_image_queue');
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

function wp_remote_get($url, $args = [])
{
    if (is_callable($GLOBALS['http_mock'])) {
        return call_user_func($GLOBALS['http_mock'], $url, $args);
    }

    return [
        'response' => ['code' => 200],
        'body' => json_encode(['status' => 'running', 'server' => 'VeloServe', 'version' => 'dev']),
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

function sanitize_email($value)
{
    return trim((string) $value);
}

function sanitize_textarea_field($value)
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

function wp_parse_url($url)
{
    return parse_url($url);
}

function wp_json_encode($value)
{
    return json_encode($value);
}

function esc_url($value)
{
    return trim((string) $value);
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
function add_filter() {}
function wp_is_post_revision() { return false; }
function get_permalink($post_id) { return 'https://example.test/post/' . $post_id; }
function wp_attachment_is_image() { return true; }
function wp_next_scheduled() { return false; }
function wp_schedule_single_event() { return true; }
function get_attached_file($attachment_id) { return __FILE__; }
function wp_get_attachment_url($attachment_id) { return 'https://example.test/uploads/image-' . (int) $attachment_id . '.jpg'; }
function get_post_meta($post_id, $key, $single = false) { return []; }
function update_post_meta($post_id, $key, $value) { return true; }
function checked($checked, $current = true, $echo = true) { return ''; }
function selected($selected, $current = true, $echo = true) { return ''; }

require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-client.php';
require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-server.php';
require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-cdn-provider.php';
require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-cdn-cloudflare-provider.php';
require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-cdn-manager.php';
require_once __DIR__ . '/../veloserve-cache/includes/class-veloserve-image-optimizer.php';
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
assert_equals(1, $settings['auto_detect_server'], 'auto_detect_server default should be enabled');
assert_equals(0, $settings['guest_mode'], 'guest_mode default should be disabled');
assert_equals('', $settings['server_ip_override'], 'server_ip_override default should be empty');
assert_equals(1, $settings['notifications_enabled'], 'notifications_enabled default should be enabled');
assert_equals(0, $settings['cdn_enabled'], 'cdn_enabled default should be disabled');
assert_equals('none', $settings['cdn_provider'], 'cdn_provider default should be none');
assert_equals('', $settings['cloudflare_zone_id'], 'cloudflare_zone_id default should be empty');
assert_equals(1, $settings['opt_minify_css'], 'opt_minify_css default should be enabled');
assert_equals(0, $settings['opt_combine_css'], 'opt_combine_css default should be disabled');
assert_equals(0, $settings['opt_critical_css'], 'opt_critical_css default should be disabled');
assert_equals(1, $settings['opt_minify_js'], 'opt_minify_js default should be enabled');
assert_equals(0, $settings['opt_combine_js'], 'opt_combine_js default should be disabled');
assert_equals(1, $settings['opt_defer_js'], 'opt_defer_js default should be enabled');
assert_equals(1, $settings['opt_minify_html'], 'opt_minify_html default should be enabled');
assert_equals(0, $settings['opt_prefetch_hints'], 'opt_prefetch_hints default should be disabled');
assert_equals('', $settings['opt_prefetch_urls'], 'opt_prefetch_urls default should be empty');
assert_equals(1, $settings['opt_lazyload_images'], 'opt_lazyload_images default should be enabled');
assert_equals(1, $settings['opt_image_webp'], 'opt_image_webp default should be enabled');
assert_equals(0, $settings['opt_image_avif'], 'opt_image_avif default should be disabled');
assert_equals(82, $settings['opt_image_quality'], 'opt_image_quality default should be 82');
assert_equals(1, $settings['opt_image_queue'], 'opt_image_queue default should be enabled');
assert_true(is_array($status), 'Activation should create status option');
assert_equals(false, $status['connected'], 'connected should default to false');

$admin = new VeloServe_Admin();
$sanitized = $admin->sanitize([
    'endpoint_url' => ' https://control.example.test/ ',
    'api_token' => ' secret-token ',
    'auto_detect_server' => '1',
    'guest_mode' => '1',
    'server_ip_override' => '203.0.113.10',
    'notifications_enabled' => '1',
    'auto_purge' => '1',
    'opt_minify_css' => '1',
    'opt_combine_css' => '1',
    'opt_critical_css' => '1',
    'opt_minify_js' => '1',
    'opt_combine_js' => '1',
    'opt_defer_js' => '1',
    'opt_minify_html' => '1',
    'opt_prefetch_hints' => '1',
    'opt_prefetch_urls' => " https://fonts.gstatic.com \nhttps://cdn.example.test/assets ",
    'opt_lazyload_images' => '1',
    'opt_image_webp' => '1',
    'opt_image_avif' => '1',
    'opt_image_quality' => '78',
    'opt_image_queue' => '1',
    'cdn_enabled' => '1',
    'cdn_provider' => 'cloudflare',
    'cloudflare_zone_id' => 'zone-123',
    'cloudflare_api_token' => 'cf-token',
    'cloudflare_email' => 'ops@example.test',
    'cloudflare_api_key' => 'legacy-key',
]);
assert_equals('https://control.example.test/', $sanitized['endpoint_url'], 'sanitize should trim endpoint_url');
assert_equals('secret-token', $sanitized['api_token'], 'sanitize should trim api_token');
assert_equals(1, $sanitized['auto_detect_server'], 'sanitize should persist auto_detect_server');
assert_equals(1, $sanitized['guest_mode'], 'sanitize should persist guest_mode');
assert_equals('203.0.113.10', $sanitized['server_ip_override'], 'sanitize should keep valid server_ip_override');
assert_equals(1, $sanitized['notifications_enabled'], 'sanitize should persist notifications_enabled');
assert_equals(1, $sanitized['auto_purge'], 'sanitize should persist auto_purge');
assert_equals(1, $sanitized['opt_minify_css'], 'sanitize should persist opt_minify_css');
assert_equals(1, $sanitized['opt_combine_css'], 'sanitize should persist opt_combine_css');
assert_equals(1, $sanitized['opt_critical_css'], 'sanitize should persist opt_critical_css');
assert_equals(1, $sanitized['opt_minify_js'], 'sanitize should persist opt_minify_js');
assert_equals(1, $sanitized['opt_combine_js'], 'sanitize should persist opt_combine_js');
assert_equals(1, $sanitized['opt_defer_js'], 'sanitize should persist opt_defer_js');
assert_equals(1, $sanitized['opt_minify_html'], 'sanitize should persist opt_minify_html');
assert_equals(1, $sanitized['opt_prefetch_hints'], 'sanitize should persist opt_prefetch_hints');
assert_equals("https://fonts.gstatic.com\nhttps://cdn.example.test/assets", $sanitized['opt_prefetch_urls'], 'sanitize should normalize opt_prefetch_urls');
assert_equals(1, $sanitized['opt_lazyload_images'], 'sanitize should persist opt_lazyload_images');
assert_equals(1, $sanitized['opt_image_webp'], 'sanitize should persist opt_image_webp');
assert_equals(1, $sanitized['opt_image_avif'], 'sanitize should persist opt_image_avif');
assert_equals(78, $sanitized['opt_image_quality'], 'sanitize should persist opt_image_quality');
assert_equals(1, $sanitized['opt_image_queue'], 'sanitize should persist opt_image_queue');
assert_equals(1, $sanitized['cdn_enabled'], 'sanitize should persist cdn_enabled');
assert_equals('cloudflare', $sanitized['cdn_provider'], 'sanitize should persist cdn_provider');
assert_equals('zone-123', $sanitized['cloudflare_zone_id'], 'sanitize should persist cloudflare_zone_id');
assert_equals('cf-token', $sanitized['cloudflare_api_token'], 'sanitize should persist cloudflare_api_token');
assert_equals('ops@example.test', $sanitized['cloudflare_email'], 'sanitize should persist cloudflare_email');
assert_equals('legacy-key', $sanitized['cloudflare_api_key'], 'sanitize should persist cloudflare_api_key');

$invalid_ip = $admin->sanitize([
    'server_ip_override' => 'bad-ip-value',
]);
assert_equals('', $invalid_ip['server_ip_override'], 'sanitize should drop invalid server_ip_override');

update_option('veloserve_settings', [
    'endpoint_url' => 'https://control.example.test',
    'api_token' => 'secret-token',
    'auto_purge' => 1,
    'auto_detect_server' => 1,
    'guest_mode' => 0,
    'server_ip_override' => '',
    'notifications_enabled' => 1,
    'opt_minify_css' => 1,
    'opt_combine_css' => 0,
    'opt_critical_css' => 0,
    'opt_minify_js' => 1,
    'opt_combine_js' => 0,
    'opt_defer_js' => 1,
    'opt_minify_html' => 1,
    'opt_prefetch_hints' => 1,
    'opt_prefetch_urls' => "https://fonts.gstatic.com\nhttps://cdn.example.test/assets",
    'opt_lazyload_images' => 1,
    'opt_image_webp' => 1,
    'opt_image_avif' => 0,
    'opt_image_quality' => 82,
    'opt_image_queue' => 1,
    'cdn_enabled' => 0,
    'cdn_provider' => 'none',
    'cloudflare_zone_id' => '',
    'cloudflare_api_token' => '',
    'cloudflare_email' => '',
    'cloudflare_api_key' => '',
]);

$plugin = VeloServe_Plugin::instance();
$plugin->bootstrap();

$registration_payload = null;
$GLOBALS['http_mock'] = function ($url, $args) use (&$registration_payload) {
    if (strpos($url, '/api/v1/wordpress/register') !== false) {
        $registration_payload = json_decode($args['body'], true);
    }

    return [
        'response' => ['code' => 201],
        'body' => json_encode(['node_id' => 'node-123']),
    ];
};

$result = $plugin->register_with_endpoint();
assert_true(!is_wp_error($result), 'Register should succeed on 2xx response');
assert_equals('node-123', get_option('veloserve_status')['node_id'], 'Node id should persist on success');
assert_equals(true, get_option('veloserve_status')['connected'], 'Connected should be true after success');
assert_true(is_array($registration_payload), 'Register payload should be captured');
assert_true(isset($registration_payload['optimization']), 'Register payload should include optimization settings');
assert_equals(true, $registration_payload['optimization']['minify_css'], 'Register payload should include minify_css');
assert_equals(true, $registration_payload['optimization']['defer_js'], 'Register payload should include defer_js');
assert_true(isset($registration_payload['optimization']['images']), 'Register payload should include image optimization settings');
assert_equals(true, $registration_payload['optimization']['images']['lazyload'], 'Register payload should include lazyload flag');
assert_equals(true, $registration_payload['optimization']['images']['webp'], 'Register payload should include webp flag');
assert_equals(false, $registration_payload['optimization']['images']['avif'], 'Register payload should include avif flag');
assert_equals(82, $registration_payload['optimization']['images']['quality'], 'Register payload should include image quality');

$GLOBALS['http_mock'] = function ($url, $args) {
    return [
        'response' => ['code' => 500],
        'body' => '{}',
    ];
};

$failure = $plugin->register_with_endpoint();
assert_true(is_wp_error($failure), 'Register should fail on non-2xx response');

$server = new VeloServe_Server();
$calls = [];
$GLOBALS['http_mock'] = function ($url, $args) use (&$calls) {
    $calls[] = ['url' => $url, 'args' => $args];

    if (strpos($url, '/api/v1/status') !== false) {
        return [
            'response' => ['code' => 200],
            'body' => json_encode([
                'status' => 'running',
                'server' => 'veloserve',
                'version' => '1.2.3',
                'php_available' => true,
                'cache_enabled' => true,
            ]),
        ];
    }

    if (strpos($url, '/api/v1/cache/stats') !== false) {
        return [
            'response' => ['code' => 200],
            'body' => json_encode([
                'cache' => ['hit_rate' => 0.91],
                'warming' => ['queued_total' => 12],
            ]),
        ];
    }

    if (strpos($url, '/api/v1/cache/purge') !== false) {
        return [
            'response' => ['code' => 200],
            'body' => json_encode(['success' => true]),
        ];
    }

    if (strpos($url, '/api/v1/cache/warm') !== false) {
        return [
            'response' => ['code' => 200],
            'body' => json_encode(['accepted' => 2, 'queued' => 2]),
        ];
    }

    if (strpos($url, '/client/v4/zones/zone-123') !== false) {
        return [
            'response' => ['code' => 200],
            'body' => json_encode([
                'success' => true,
                'result' => ['name' => 'example.test'],
            ]),
        ];
    }

    return [
        'response' => ['code' => 404],
        'body' => '{}',
    ];
};

$detected = $server->detect_server([
    'endpoint_url' => 'http://127.0.0.1:8080',
    'api_token' => 'secret-token',
]);
assert_true(!is_wp_error($detected), 'Server detection should succeed');
assert_equals('1.2.3', $detected['version'], 'Detected version should be parsed');

$stats = $server->get_cache_stats([
    'endpoint_url' => 'http://127.0.0.1:8080',
    'api_token' => 'secret-token',
]);
assert_true(!is_wp_error($stats), 'Cache stats should succeed');
assert_true(isset($stats['cache']['hit_rate']), 'Cache stats should include hit_rate');

$purge = $server->purge_cache(
    ['endpoint_url' => 'http://127.0.0.1:8080', 'api_token' => 'secret-token'],
    ['url' => 'https://example.test/post/42']
);
assert_true(!is_wp_error($purge), 'Purge should succeed');

$warm = $server->warm_cache(
    ['endpoint_url' => 'http://127.0.0.1:8080', 'api_token' => 'secret-token'],
    ['https://example.test/uploads/image-12.jpg', 'https://example.test/uploads/image-12.webp'],
    'wordpress-image-opt',
    'manual'
);
assert_true(!is_wp_error($warm), 'Cache warm should succeed');

$last_call = $calls[count($calls) - 1];
assert_true(
    strpos($last_call['url'], '/api/v1/cache/warm') !== false,
    'Warm URL should target cache warm endpoint'
);
assert_true(
    strpos($last_call['args']['body'], 'wordpress-image-opt') !== false,
    'Warm request body should include trigger'
);

$cdn_manager = new VeloServe_CDN_Manager();
$cdn_test = $cdn_manager->test_connection([
    'cdn_provider' => 'cloudflare',
    'cdn_enabled' => 1,
    'cloudflare_zone_id' => 'zone-123',
    'cloudflare_api_token' => 'cf-token',
]);
assert_true(!is_wp_error($cdn_test), 'CDN connection test should succeed with Cloudflare settings');
assert_equals('example.test', $cdn_test['zone_name'], 'CDN connection test should parse zone name');

$request_urls = [];
$GLOBALS['http_mock'] = function ($url, $args) use (&$request_urls) {
    $request_urls[] = $url;
    return [
        'response' => ['code' => 200],
        'body' => json_encode(['success' => true]),
    ];
};

$post = new WP_Post('publish');
$plugin->purge_cache_on_content_change(42, $post);
$purge_urls = array_values(array_filter($request_urls, function ($url) {
    return strpos($url, '/api/v1/cache/purge') !== false;
}));
assert_true(count($purge_urls) >= 2, 'Content change should trigger targeted purge requests');
assert_true(
    strpos(implode("\n", $purge_urls), 'domain=example.test&path=%2Fpost%2F42') !== false,
    'Content purge should include changed post path'
);

$request_urls = [];
$plugin->purge_cache_on_switch_theme();
$purge_urls = array_values(array_filter($request_urls, function ($url) {
    return strpos($url, '/api/v1/cache/purge') !== false;
}));
assert_true(count($purge_urls) >= 2, 'Theme switch should trigger targeted purge requests');
assert_true(
    strpos(implode("\n", $purge_urls), 'domain=example.test&path=%2Fwp-json%2F') !== false,
    'Theme switch purge should include REST index path'
);

update_option('veloserve_settings', [
    'endpoint_url' => 'https://control.example.test',
    'api_token' => 'secret-token',
    'auto_purge' => 1,
    'auto_detect_server' => 1,
    'guest_mode' => 0,
    'server_ip_override' => '',
    'notifications_enabled' => 1,
    'cdn_enabled' => 1,
    'cdn_provider' => 'cloudflare',
    'cloudflare_zone_id' => 'zone-123',
    'cloudflare_api_token' => 'cf-token',
    'cloudflare_email' => '',
    'cloudflare_api_key' => '',
]);

$request_urls = [];
$plugin->purge_cache_on_plugin_change();
$cloudflare_calls = array_values(array_filter($request_urls, function ($url) {
    return strpos($url, 'https://api.cloudflare.com/client/v4/zones/zone-123/purge_cache') !== false;
}));
assert_true(count($cloudflare_calls) >= 1, 'Plugin lifecycle purge should cascade to Cloudflare when CDN is enabled');

$request_urls = [];
$plugin->purge_cache_on_plugin_change();
$purge_urls = array_values(array_filter($request_urls, function ($url) {
    return strpos($url, '/api/v1/cache/purge') !== false;
}));
assert_true(count($purge_urls) >= 2, 'Plugin lifecycle changes should trigger targeted purge requests');

$request_urls = [];
$plugin->purge_cache_on_order_status_change(1001, 'pending', 'processing', null);
$purge_urls = array_values(array_filter($request_urls, function ($url) {
    return strpos($url, '/api/v1/cache/purge') !== false;
}));
assert_true(count($purge_urls) >= 5, 'Commerce order events should trigger storefront purge requests');
assert_true(
    strpos(implode("\n", $purge_urls), 'domain=example.test&path=%2Fcheckout%2F') !== false,
    'Commerce purge should include checkout path'
);

$request_urls = [];
update_option('veloserve_settings', [
    'endpoint_url' => 'https://control.example.test',
    'api_token' => 'secret-token',
    'auto_purge' => 0,
    'auto_detect_server' => 1,
    'guest_mode' => 0,
    'server_ip_override' => '',
    'notifications_enabled' => 1,
]);
$plugin->purge_cache_on_content_change(43, $post);
$purge_urls = array_values(array_filter($request_urls, function ($url) {
    return strpos($url, '/api/v1/cache/purge') !== false;
}));
assert_true(count($purge_urls) === 0, 'Content change should not purge when auto_purge is disabled');

VeloServe_Plugin::deactivate();
assert_equals(false, get_option('veloserve_status')['connected'], 'Deactivate should set connected=false');

echo "Plugin flow tests passed.\n";
