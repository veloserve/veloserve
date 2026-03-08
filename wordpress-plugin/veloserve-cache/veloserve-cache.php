<?php
/**
 * Plugin Name: VeloServe Cache
 * Description: Connects WordPress to VeloServe for cache controls and node registration.
 * Version: 0.1.0
 * Author: VeloServe
 * License: GPL-2.0-or-later
 * Requires at least: 6.0
 * Requires PHP: 7.4
 */

if (!defined('ABSPATH')) {
    exit;
}

define('VELOSERVE_PLUGIN_VERSION', '0.1.0');
define('VELOSERVE_PLUGIN_FILE', __FILE__);
define('VELOSERVE_PLUGIN_DIR', plugin_dir_path(__FILE__));

define('VELOSERVE_OPTION_KEY', 'veloserve_settings');
define('VELOSERVE_STATUS_KEY', 'veloserve_status');

require_once VELOSERVE_PLUGIN_DIR . 'includes/class-veloserve-client.php';
require_once VELOSERVE_PLUGIN_DIR . 'includes/class-veloserve-server.php';
require_once VELOSERVE_PLUGIN_DIR . 'includes/class-veloserve-cdn-provider.php';
require_once VELOSERVE_PLUGIN_DIR . 'includes/class-veloserve-cdn-cloudflare-provider.php';
require_once VELOSERVE_PLUGIN_DIR . 'includes/class-veloserve-cdn-manager.php';
require_once VELOSERVE_PLUGIN_DIR . 'includes/class-veloserve-admin.php';
require_once VELOSERVE_PLUGIN_DIR . 'includes/class-veloserve-plugin.php';

register_activation_hook(__FILE__, ['VeloServe_Plugin', 'activate']);
register_deactivation_hook(__FILE__, ['VeloServe_Plugin', 'deactivate']);

VeloServe_Plugin::instance()->bootstrap();
