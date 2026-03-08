<?php

if (!defined('ABSPATH')) {
    exit;
}

interface VeloServe_CDN_Provider
{
    public function provider_key();

    public function test_connection(array $settings);

    public function purge(array $settings, array $params = []);
}
