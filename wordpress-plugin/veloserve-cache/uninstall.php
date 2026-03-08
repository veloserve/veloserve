<?php

if (!defined('WP_UNINSTALL_PLUGIN')) {
    exit;
}

delete_option('veloserve_settings');
delete_option('veloserve_status');
delete_option('veloserve_image_queue');
