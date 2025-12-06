<?php
/**
 * VeloServe PHP Test Page
 */

// Set headers
header('Content-Type: text/html; charset=utf-8');
header('X-Powered-By: VeloServe');

// Get server info
$serverInfo = [
    'PHP Version' => phpversion(),
    'Server Software' => $_SERVER['SERVER_SOFTWARE'] ?? 'Unknown',
    'Server Name' => $_SERVER['SERVER_NAME'] ?? 'Unknown',
    'Request Method' => $_SERVER['REQUEST_METHOD'] ?? 'Unknown',
    'Request URI' => $_SERVER['REQUEST_URI'] ?? 'Unknown',
    'Document Root' => $_SERVER['DOCUMENT_ROOT'] ?? 'Unknown',
    'Script Filename' => $_SERVER['SCRIPT_FILENAME'] ?? 'Unknown',
];

?>
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>VeloServe PHP Test</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: 'JetBrains Mono', 'Fira Code', monospace;
            background: #0d1117;
            color: #c9d1d9;
            padding: 2rem;
            line-height: 1.6;
        }
        
        .container {
            max-width: 800px;
            margin: 0 auto;
        }
        
        h1 {
            color: #58a6ff;
            margin-bottom: 1rem;
            font-size: 1.5rem;
        }
        
        .success {
            background: #238636;
            color: white;
            padding: 1rem;
            border-radius: 6px;
            margin-bottom: 2rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }
        
        .info-table {
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 2rem;
            background: #161b22;
            border-radius: 6px;
            overflow: hidden;
        }
        
        .info-table th,
        .info-table td {
            padding: 0.75rem 1rem;
            text-align: left;
            border-bottom: 1px solid #30363d;
        }
        
        .info-table th {
            background: #21262d;
            color: #58a6ff;
            font-weight: 600;
        }
        
        .info-table tr:last-child td {
            border-bottom: none;
        }
        
        .info-table td:first-child {
            color: #7ee787;
            width: 40%;
        }
        
        code {
            background: #21262d;
            padding: 0.2rem 0.5rem;
            border-radius: 4px;
            font-size: 0.9rem;
        }
        
        .timestamp {
            color: #8b949e;
            font-size: 0.9rem;
            margin-top: 2rem;
        }
        
        a {
            color: #58a6ff;
            text-decoration: none;
        }
        
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>⚡ VeloServe PHP Integration Test</h1>
        
        <div class="success">
            ✓ PHP is working correctly!
        </div>
        
        <h2 style="margin-bottom: 1rem; color: #58a6ff;">Server Information</h2>
        <table class="info-table">
            <tbody>
                <?php foreach ($serverInfo as $key => $value): ?>
                <tr>
                    <td><?= htmlspecialchars($key) ?></td>
                    <td><code><?= htmlspecialchars($value) ?></code></td>
                </tr>
                <?php endforeach; ?>
            </tbody>
        </table>
        
        <h2 style="margin-bottom: 1rem; color: #58a6ff;">PHP Extensions</h2>
        <table class="info-table">
            <tbody>
                <?php 
                $extensions = ['json', 'mbstring', 'openssl', 'pdo', 'curl', 'gd', 'xml'];
                foreach ($extensions as $ext): 
                    $loaded = extension_loaded($ext);
                ?>
                <tr>
                    <td><?= $ext ?></td>
                    <td>
                        <?php if ($loaded): ?>
                            <span style="color: #7ee787;">✓ Loaded</span>
                        <?php else: ?>
                            <span style="color: #f85149;">✗ Not loaded</span>
                        <?php endif; ?>
                    </td>
                </tr>
                <?php endforeach; ?>
            </tbody>
        </table>
        
        <p class="timestamp">
            Generated at: <?= date('Y-m-d H:i:s') ?> |
            <a href="/">← Back to Home</a> |
            <a href="/info.php">Full PHP Info</a>
        </p>
    </div>
</body>
</html>

