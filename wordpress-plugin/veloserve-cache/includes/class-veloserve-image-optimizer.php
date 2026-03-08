<?php

if (!defined('ABSPATH')) {
    exit;
}

class VeloServe_Image_Optimizer
{
    private $server;

    public function __construct($server = null)
    {
        $this->server = $server ?: new VeloServe_Server();
    }

    public function hooks()
    {
        add_filter('wp_get_attachment_image_attributes', [$this, 'apply_lazyload_attributes'], 10, 3);
        add_filter('wp_get_attachment_image', [$this, 'wrap_attachment_image'], 10, 5);
        add_action('add_attachment', [$this, 'enqueue_attachment']);
        add_action('veloserve_image_optimize_queue', [$this, 'process_queue']);
    }

    public function apply_lazyload_attributes($attr, $attachment, $size)
    {
        $settings = $this->settings();
        if (empty($settings['opt_lazyload_images'])) {
            return is_array($attr) ? $attr : [];
        }

        if (!is_array($attr)) {
            $attr = [];
        }

        if (empty($attr['loading'])) {
            $attr['loading'] = 'lazy';
        }

        if (empty($attr['decoding'])) {
            $attr['decoding'] = 'async';
        }

        if (empty($attr['fetchpriority'])) {
            $attr['fetchpriority'] = 'low';
        }

        return $attr;
    }

    public function wrap_attachment_image($html, $attachment_id, $size, $icon, $attr)
    {
        if (!is_string($html) || trim($html) === '' || strpos($html, '<picture') !== false) {
            return $html;
        }

        $settings = $this->settings();
        $formats = $this->get_generated_formats((int) $attachment_id);
        if (!$formats) {
            return $html;
        }

        $sources = [];
        if (!empty($settings['opt_image_avif']) && !empty($formats['avif'])) {
            $sources[] = '<source type="image/avif" srcset="' . esc_url($formats['avif']) . '" />';
        }
        if (!empty($settings['opt_image_webp']) && !empty($formats['webp'])) {
            $sources[] = '<source type="image/webp" srcset="' . esc_url($formats['webp']) . '" />';
        }

        if (empty($sources)) {
            return $html;
        }

        return '<picture>' . implode('', $sources) . $html . '</picture>';
    }

    public function enqueue_attachment($attachment_id)
    {
        $settings = $this->settings();
        if (empty($settings['opt_image_queue'])) {
            return;
        }

        $attachment_id = (int) $attachment_id;
        if ($attachment_id <= 0 || !$this->is_image_attachment($attachment_id)) {
            return;
        }

        $queue = get_option(VELOSERVE_IMAGE_QUEUE_KEY, []);
        if (!is_array($queue)) {
            $queue = [];
        }

        if (!in_array($attachment_id, $queue, true)) {
            $queue[] = $attachment_id;
            update_option(VELOSERVE_IMAGE_QUEUE_KEY, $queue);
        }

        if (function_exists('wp_next_scheduled') && function_exists('wp_schedule_single_event')) {
            if (!wp_next_scheduled('veloserve_image_optimize_queue')) {
                wp_schedule_single_event(time() + 5, 'veloserve_image_optimize_queue');
            }
        }
    }

    public function process_queue()
    {
        $settings = $this->settings();
        if (empty($settings['opt_image_queue'])) {
            return;
        }

        $queue = get_option(VELOSERVE_IMAGE_QUEUE_KEY, []);
        if (!is_array($queue) || empty($queue)) {
            return;
        }

        $batch_size = 5;
        $batch = array_slice($queue, 0, $batch_size);
        $remaining = array_slice($queue, $batch_size);

        foreach ($batch as $attachment_id) {
            $this->optimize_attachment((int) $attachment_id, $settings);
        }

        update_option(VELOSERVE_IMAGE_QUEUE_KEY, $remaining);
        if (!empty($remaining) && function_exists('wp_schedule_single_event')) {
            wp_schedule_single_event(time() + 10, 'veloserve_image_optimize_queue');
        }
    }

    private function optimize_attachment($attachment_id, array $settings)
    {
        if ($attachment_id <= 0) {
            return;
        }

        if (!function_exists('get_attached_file') || !function_exists('wp_get_attachment_url')) {
            return;
        }

        $file_path = get_attached_file($attachment_id);
        if (!is_string($file_path) || $file_path === '' || !file_exists($file_path)) {
            return;
        }

        $quality = isset($settings['opt_image_quality']) ? (int) $settings['opt_image_quality'] : 82;
        if ($quality < 30) {
            $quality = 30;
        } elseif ($quality > 100) {
            $quality = 100;
        }

        $this->compress_original($file_path, $quality);

        $generated = [];
        if (!empty($settings['opt_image_webp'])) {
            $generated_url = $this->generate_format($attachment_id, $file_path, $quality, 'webp', 'image/webp');
            if ($generated_url !== '') {
                $generated['webp'] = $generated_url;
            }
        }

        if (!empty($settings['opt_image_avif'])) {
            $generated_url = $this->generate_format($attachment_id, $file_path, $quality, 'avif', 'image/avif');
            if ($generated_url !== '') {
                $generated['avif'] = $generated_url;
            }
        }

        if (empty($generated)) {
            return;
        }

        if (function_exists('update_post_meta')) {
            update_post_meta($attachment_id, '_veloserve_generated_formats', $generated);
        }

        $source_url = wp_get_attachment_url($attachment_id);
        if (!is_string($source_url) || $source_url === '') {
            return;
        }

        $urls = [$source_url];
        foreach ($generated as $generated_url) {
            if (is_string($generated_url) && $generated_url !== '') {
                $urls[] = $generated_url;
            }
        }

        $this->server->warm_cache($settings, $urls, 'wordpress-image-opt', 'manual');
    }

    private function compress_original($file_path, $quality)
    {
        if (!function_exists('wp_get_image_editor')) {
            return;
        }

        $editor = wp_get_image_editor($file_path);
        if (is_wp_error($editor)) {
            return;
        }

        if (is_object($editor) && method_exists($editor, 'set_quality')) {
            $editor->set_quality((int) $quality);
        }

        if (is_object($editor) && method_exists($editor, 'save')) {
            $editor->save($file_path);
        }
    }

    private function generate_format($attachment_id, $source_path, $quality, $extension, $mime_type)
    {
        if (!function_exists('wp_get_image_editor') || !function_exists('wp_get_attachment_url')) {
            return '';
        }

        $editor = wp_get_image_editor($source_path);
        if (is_wp_error($editor)) {
            return '';
        }

        if (is_object($editor) && method_exists($editor, 'set_quality')) {
            $editor->set_quality((int) $quality);
        }

        $target_path = preg_replace('/\.[^.]+$/', '.' . $extension, (string) $source_path);
        if (!is_string($target_path) || $target_path === '') {
            return '';
        }

        $result = is_object($editor) && method_exists($editor, 'save')
            ? $editor->save($target_path, $mime_type)
            : new WP_Error('veloserve_no_editor_save', 'Image editor save method is unavailable.');
        if (is_wp_error($result)) {
            return '';
        }

        $source_url = wp_get_attachment_url($attachment_id);
        if (!is_string($source_url) || $source_url === '') {
            return '';
        }

        return preg_replace('/\.[^.]+$/', '.' . $extension, $source_url);
    }

    private function get_generated_formats($attachment_id)
    {
        if (!function_exists('get_post_meta')) {
            return [];
        }

        $formats = get_post_meta((int) $attachment_id, '_veloserve_generated_formats', true);
        if (!is_array($formats)) {
            return [];
        }

        return $formats;
    }

    private function is_image_attachment($attachment_id)
    {
        if (function_exists('wp_attachment_is_image')) {
            return (bool) wp_attachment_is_image($attachment_id);
        }

        if (function_exists('get_post_mime_type')) {
            $mime = (string) get_post_mime_type($attachment_id);
            return strpos($mime, 'image/') === 0;
        }

        return true;
    }

    private function settings()
    {
        $settings = get_option(VELOSERVE_OPTION_KEY, VeloServe_Plugin::default_settings());
        return array_merge(VeloServe_Plugin::default_settings(), is_array($settings) ? $settings : []);
    }
}
