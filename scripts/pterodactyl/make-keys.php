<?php
$user = Pterodactyl\Models\User::where('email', 'admin@ptero.test')->firstOrFail();
$keys = app(Pterodactyl\Services\Api\KeyCreationService::class);
$resources = ['r_users', 'r_servers', 'r_nests', 'r_eggs', 'r_locations', 'r_nodes', 'r_allocations', 'r_database_hosts', 'r_server_databases'];
$application = $keys->setKeyType(Pterodactyl\Models\ApiKey::TYPE_APPLICATION)->handle(
    ['user_id' => $user->id, 'memo' => 'stacklane', 'allowed_ips' => []],
    array_fill_keys($resources, 3)
);
$client = $keys->setKeyType(Pterodactyl\Models\ApiKey::TYPE_ACCOUNT)->handle(
    ['user_id' => $user->id, 'memo' => 'stacklane', 'allowed_ips' => []]
);
echo 'PTERODACTYL_APPLICATION_KEY=' . $application->identifier . decrypt($application->token) . PHP_EOL;
echo 'PTERODACTYL_CLIENT_KEY=' . $client->identifier . decrypt($client->token) . PHP_EOL;
